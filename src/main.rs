use std::{io::Write, path::Path, sync::Arc, time::Duration};

use notify::RecursiveMode;
use tokio::runtime::Runtime;

use crate::compiler::{CompilerState, compile};

pub mod arena;
pub mod ast;
pub mod compiler;
pub mod errors;
pub mod file;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod resolve;
pub mod types;

fn clear() {
    print!("\x1B[2J\x1B[1;1H");
    std::io::stdout().flush().unwrap();
}

fn live(filename: &str) {
    let runtime = Runtime::new().expect("unable to create runtime");
    let state = Arc::new(CompilerState::default());
    let filename: Arc<str> = filename.into();
    let filename_clone = filename.clone();
    let mut debouncer =
        notify_debouncer_mini::new_debouncer(Duration::from_millis(100), move |ev| match ev {
            Ok(_) => {
                clear();
                runtime.block_on(async {
                    compile(None, &filename_clone, state.clone()).await;

                    let files = state.files.lock().await;
                    let ast = state.ast.lock().await;
                    let symbols = state.symbols.lock().await;
                    let types = state.types.lock().await;
                    let root = state.root.lock().await;

                    if let Some(root) = *root {
                        ast.pretty_print(root, &files);
                        symbols.pretty_print(root, &ast, &files);
                        types.pretty_print(&ast, &symbols);
                    }
                });
            }
            Err(e) => eprintln!("{}", e),
        })
        .unwrap();
    debouncer
        .watcher()
        .watch(Path::new(filename.as_ref()), RecursiveMode::Recursive)
        .unwrap();

    clear();
    std::thread::sleep(Duration::MAX);
}

fn main() {
    let mut args = std::env::args();
    if args.len() <= 1 {
        println!("error: expected at least one argument");
        return;
    }
    let cmd = args.nth(1).unwrap();
    match cmd.as_str() {
        "live" => {
            live(&args.next().expect("missing file name"));
        }
        "lsp" => {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(lsp::lsp_main());
        }
        _ => {
            println!("unrecognized command. valid commands are `live` and `lsp`");
        }
    }
}
