use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};

use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::request::*;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use crate::ast::AstId;
use crate::ast::{Ast, AstKind};
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

static TOKEN_TYPES: LazyLock<Vec<SemanticTokenType>> = LazyLock::new(|| {
    vec![
        SemanticTokenType::NAMESPACE,
        SemanticTokenType::TYPE,
        SemanticTokenType::CLASS,
        SemanticTokenType::ENUM,
        SemanticTokenType::INTERFACE,
        SemanticTokenType::STRUCT,
        SemanticTokenType::TYPE_PARAMETER,
        SemanticTokenType::PARAMETER,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::EVENT,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::METHOD,
        SemanticTokenType::MACRO,
        SemanticTokenType::KEYWORD,
        SemanticTokenType::MODIFIER,
        SemanticTokenType::COMMENT,
        SemanticTokenType::STRING,
        SemanticTokenType::NUMBER,
        SemanticTokenType::REGEXP,
        SemanticTokenType::OPERATOR,
        SemanticTokenType::DECORATOR,
    ]
});
static TOKEN_MAP: LazyLock<HashMap<SemanticTokenType, u32>> = LazyLock::new(|| {
    let mut result = HashMap::new();
    for (index, token_type) in TOKEN_TYPES.iter().enumerate() {
        result.insert(token_type.clone(), index as u32);
    }
    result
});

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
                                token_types: TOKEN_TYPES.clone(),
                                token_modifiers: vec![],
                            },
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                definition_provider: Some(OneOf::Left(true)),
                type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
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
                contents: HoverContents::Scalar(MarkedString::LanguageString(LanguageString {
                    language: String::from("ket"),
                    value: String::from("internal error: no type assignment"),
                })),
                range: None,
            }));
        };

        let expand = if let Some(def_id) = tid.get(&types.definitions) {
            def_id.get(&symbols.qualified_idents) == hovered_id.get(&symbols.qualified_idents)
        } else {
            false
        };
        let msg = types.string_of_type(*tid, expand, &ast);
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::LanguageString(LanguageString {
                language: String::from("ket"),
                value: msg,
            })),
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
                    character: loc.char_end - 1,
                },
            },
        })))
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> Result<Option<GotoTypeDefinitionResponse>> {
        let ast = self.state.ast.lock().await;
        let types = self.state.types.lock().await;

        let Some(id) = find_ident(params.text_document_position_params.position, &ast) else {
            return Ok(None);
        };
        let Some(tid) = id.get(&types.assignments) else {
            return Ok(None);
        };
        let Some(def_id) = tid.get(&types.definitions) else {
            return Ok(None);
        };
        let Some(loc) = def_id.get(&ast.locations) else {
            return Ok(None);
        };
        Ok(Some(GotoTypeDefinitionResponse::Scalar(Location {
            uri: params.text_document_position_params.text_document.uri,
            range: Range {
                start: Position {
                    line: loc.line_start - 1,
                    character: loc.char_start - 1,
                },
                end: Position {
                    line: loc.line_end - 1,
                    character: loc.char_end - 1,
                },
            },
        })))
    }

    async fn semantic_tokens_full(
        &self,
        _: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let mut tokens = Vec::new();
        let ast = self.state.ast.lock().await;

        let mut ids: Vec<_> = ast
            .ids
            .iter()
            .filter_map(|id| id.get(&ast.locations).map(|loc| (id, loc)))
            .collect();
        ids.sort_by(|x, y| match x.1.line_start.cmp(&y.1.line_start) {
            std::cmp::Ordering::Less => std::cmp::Ordering::Less,
            std::cmp::Ordering::Equal => x.1.char_start.cmp(&y.1.char_start),
            std::cmp::Ordering::Greater => std::cmp::Ordering::Greater,
        });

        let mut prev_line = 0;
        let mut prev_char = 0;
        for (id, location) in ids {
            let kind = *match id.get(&ast.kinds) {
                AstKind::VIdent => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                AstKind::TIdent => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::Void => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                AstKind::String => TOKEN_MAP.get(&SemanticTokenType::STRING).unwrap(),
                AstKind::Char => TOKEN_MAP.get(&SemanticTokenType::STRING).unwrap(),
                AstKind::None => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::Integer => TOKEN_MAP.get(&SemanticTokenType::NUMBER).unwrap(),
                AstKind::Float => TOKEN_MAP.get(&SemanticTokenType::NUMBER).unwrap(),
                // AstKind::Call => TOKEN_MAP.get(&SemanticTokenType::FUNCTION).unwrap(),
                // AstKind::Method => TOKEN_MAP.get(&SemanticTokenType::FUNCTION).unwrap(),
                // AstKind::Group => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Func => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Block => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Proj => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Index => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Struct => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Tuple => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Array => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Repeat => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Vector => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                AstKind::VField => TOKEN_MAP.get(&SemanticTokenType::PROPERTY).unwrap(),
                AstKind::TField => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::VArg => TOKEN_MAP.get(&SemanticTokenType::PROPERTY).unwrap(),
                AstKind::TArg => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::Bind => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::BindMut => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Assign => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::If => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::IfElse => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                // AstKind::Infix(infix_kind) => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                AstKind::BuiltinI32 => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::BuiltinF32 => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::BuiltinString => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::BuiltinChar => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::BuiltinBool => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::BuiltinTrue => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                AstKind::BuiltinFalse => TOKEN_MAP.get(&SemanticTokenType::TYPE).unwrap(),
                // AstKind::Error => TOKEN_MAP.get(&SemanticTokenType::VARIABLE).unwrap(),
                _ => continue,
            };

            let line = location.line_start - 1;
            let char = location.char_start - 1;
            let delta_line = line - prev_line;
            let delta_start = if delta_line > 0 {
                char
            } else {
                char - prev_char
            };
            tokens.push(SemanticToken {
                delta_line,
                delta_start,
                length: 1 + location.char_end - location.char_start,
                token_type: kind,
                token_modifiers_bitset: 0,
            });

            prev_line = line;
            prev_char = char;
        }
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let ast = self.state.ast.lock().await;
        let symbols = self.state.symbols.lock().await;

        let Some(hovered_id) = find_ident(params.text_document_position_params.position, &ast)
        else {
            return Ok(None);
        };
        let hovered_qualified = hovered_id.get(&symbols.qualified_idents);
        if hovered_qualified.is_empty()
            || hovered_qualified
                .split('.')
                .next_back()
                .unwrap()
                .contains("__blk")
        {
            return Ok(None);
        }

        let mut result = Vec::new();
        for id in ast.ids.iter() {
            if let Some(location) = id.get(&ast.locations)
                && id.get(&symbols.qualified_idents) == hovered_qualified
            {
                result.push(DocumentHighlight {
                    range: Range {
                        start: Position {
                            line: location.line_start - 1,
                            character: location.char_start - 1,
                        },
                        end: Position {
                            line: location.line_end - 1,
                            character: location.char_end,
                        },
                    },
                    kind: None,
                });
            }
        }

        Ok(Some(result))
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
