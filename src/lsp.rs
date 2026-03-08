use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use crate::ast::Ast;
use crate::ast::AstId;
use crate::compiler::{CompilerState, compile};

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
                        character: location.char_end,
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

    async fn refresh(&self, uri: Uri, contents: String) {
        if self
            .recompile_debounce
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        compile(
            Some(contents),
            uri.to_file_path().unwrap().to_str().unwrap(),
            self.state.clone(),
        )
        .await;
        self.compilation_counter.fetch_add(1, Ordering::Relaxed);
        self.recompile_debounce.store(false, Ordering::SeqCst);
        self.publish_diagnostics(&uri).await;
        self.client.semantic_tokens_refresh().await.ok();
    }
}

#[allow(clippy::int_plus_one)]
fn find_ident(position: Position, ast: &Ast) -> Option<AstId> {
    let mut found_id: Option<AstId> = None;
    let mut old_size = u32::MAX;
    for id in ast.ids.iter() {
        if let Some(location) = id.get(&ast.locations)
            && position.line + 1 >= location.line_start
            && position.line + 1 <= location.line_end
            && position.character + 1 >= location.char_start
            && position.character + 1 <= location.char_end
        {
            let size = location.end - location.start;
            if size < old_size {
                found_id = Some(id);
                old_size = size;
            }
        }
    }
    found_id
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
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec!["type".into(), "class".into(), "enum".into()],
                                token_modifiers: vec![],
                            },
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                definition_provider: Some(OneOf::Left(true)),
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

    async fn hover(&self, hover_params: HoverParams) -> Result<Option<Hover>> {
        let position = hover_params.text_document_position_params.position;

        let ast = self.state.ast.lock().await;
        let symbols = self.state.symbols.lock().await;
        let types = self.state.types.lock().await;

        let Some(hovered_id) = find_ident(position, &ast) else {
            return Ok(None);
        };
        let Some(tid) = hovered_id.get(&types.assignments) else {
            return Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(String::from(
                    "internal error: no type assignment",
                ))),
                range: None,
            }));
        };

        let ident = hovered_id.get(&symbols.qualified_idents);

        let msg = if let Some(hovered_def_id) = hovered_id.get(&symbols.definitions) {
            let mut expand = false;
            if *hovered_def_id == hovered_id {
                expand = true;
            }
            format!(
                "`{} {}`",
                &ident[1.min(ident.len())..],
                types.string_of_type(*tid, expand, &ast)
            )
        } else {
            format!("unable to find definition: \"{}\"", ident)
        };
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(msg)),
            range: None,
        }))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.refresh(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.refresh(
            params.text_document.uri,
            params.content_changes[0].text.clone(),
        )
        .await;
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let ast = self.state.ast.lock().await;
        let symbols = self.state.symbols.lock().await;

        let Some(id) = find_ident(params.text_document_position_params.position, &ast) else {
            return Ok(None);
        };

        let Some(definition) = id.get(&symbols.definitions) else {
            return Ok(None);
        };
        let Some(loc) = definition.get(&ast.locations) else {
            return Ok(None);
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: params.text_document_position_params.text_document.uri,
            range: Range {
                start: Position {
                    line: loc.line_start - 1,
                    character: loc.char_start - 1,
                },
                end: Position {
                    line: loc.line_end - 1,
                    character: loc.line_end - 1,
                },
            },
        })))
    }
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
