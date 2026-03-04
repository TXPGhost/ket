use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tokio::sync::Mutex;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use crate::ast::AstId;
use crate::{
    ast::Ast,
    error::Errors,
    file::{Files, read_file},
    lexer::lex_file,
    parser::Parser,
    prelude::StandardPrelude,
    types::Types,
};

#[derive(Debug)]
struct Backend {
    client: Client,
    state: Arc<CompilerState>,
    recompile_debounce: Arc<AtomicBool>,
    compilation_counter: AtomicUsize,
}

impl Backend {
    async fn publish_diagnostics(&self, uri: &Uri) {
        let errors = self.state.errors.lock().await;
        let mut diagnostics = Vec::new();

        for id in errors.ids.iter() {
            let mut diagnostic = Diagnostic::default();
            if let Some(location) = id.get(&errors.locations) {
                diagnostic.range = Range {
                    start: Position {
                        line: location.line_start - 1,
                        character: location.char_start - 1,
                    },
                    end: Position {
                        line: location.line_end - 1,
                        character: location.char_end - 1,
                    },
                };
            }
            diagnostic.severity = Some(DiagnosticSeverity::ERROR);
            diagnostic.message = id.get(&errors.messages).clone();
            diagnostic.source = Some("Ket Compiler".to_string());
            diagnostics.push(diagnostic);
        }

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    async fn recompile(&self, uri: Uri) {
        if self
            .recompile_debounce
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        let filename = uri.to_file_path().unwrap().to_str().unwrap().to_owned();
        compile_file(filename, self.state.clone()).await;
        self.compilation_counter.fetch_add(1, Ordering::Relaxed);
        self.recompile_debounce.store(false, Ordering::SeqCst);
        self.publish_diagnostics(&uri).await;
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("ket diagnostics".to_owned()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(false),
                        },
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Ket Language Server".to_owned(),
                version: Some("Debug Build".to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }

    #[allow(clippy::int_plus_one)]
    async fn hover(&self, hover_params: HoverParams) -> Result<Option<Hover>> {
        let position = hover_params.text_document_position_params.position;

        let ast = self.state.ast.lock().await;
        let types = self.state.types.lock().await;

        let mut found_id: Option<(AstId, u32)> = None;
        for id in ast.ids.iter() {
            if let Some(location) = id.get(&ast.locations)
                && position.line + 1 >= location.line_start
                && position.line + 1 <= location.line_end
                && position.character + 1 >= location.char_start
                && position.character + 1 <= location.char_end
            {
                let size = location.end - location.start;
                match found_id {
                    Some(found_id) if found_id.1 < size => {}
                    _ => {
                        found_id = Some((id, size));
                    }
                }
                break;
            }
        }
        let tid = found_id
            .map(|(id, _)| id.get(&types.assignments))
            .copied()
            .flatten();
        let ty = tid.map(|tid| tid.get(&types.types));

        let msg = match ty {
            Some(ty) => format!("{ty:?}"),
            None => match self.recompile_debounce.load(Ordering::SeqCst) {
                true => "recompiling...".to_owned(),
                false => format!(
                    "compiled {} times ({} errors)",
                    self.compilation_counter.load(Ordering::Relaxed),
                    self.state.errors.lock().await.ids.iter().count(),
                ),
            },
        };
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(msg)),
            range: None,
        }))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.recompile(params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.recompile(params.text_document.uri).await;
    }
}

#[derive(Debug, Default)]
struct CompilerState {
    errors: Mutex<Errors>,
    files: Mutex<Files>,
    ast: Mutex<Ast>,
    types: Mutex<Types>,
}

async fn compile_file(filename: String, state: Arc<CompilerState>) {
    let mut errors = Errors::default();
    let mut files = Files::default();
    let mut ast = Ast::default();
    let mut types = Types::default();

    let file = read_file(filename.as_ref(), &mut files, &mut errors);
    if let Some(file) = file {
        let tokens = lex_file(file, &files, &mut errors);
        let root = Parser::new(&tokens, &mut ast, &mut errors).parse();
        ast.simplify(root);
        ast.parse_literals(&files, &mut errors);
        ast.qualify_and_resolve(&files, root, StandardPrelude);
        types.compute_types(&ast, &mut errors);
    }

    *state.errors.lock().await = errors;
    *state.files.lock().await = files;
    *state.ast.lock().await = ast;
    *state.types.lock().await = types;
}

pub async fn lsp_main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        state: Arc::new(CompilerState::default()),
        recompile_debounce: Arc::new(AtomicBool::new(false)),
        compilation_counter: AtomicUsize::new(0),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
