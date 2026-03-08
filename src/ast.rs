pub mod kinds;
pub use kinds::*;

use smallvec::SmallVec;

use crate::{
    arena::{Arena, Id, World},
    lexer::Location,
};

#[derive(Default, Debug)]
pub struct Ast {
    pub ids: World<Ast>,
    pub kinds: Arena<Ast, AstKind>,
    pub children: Arena<Ast, SmallVec<[AstId; 4]>>,
    pub locations: Arena<Ast, Option<Location>>,
    pub idents: Arena<Ast, String>,
    pub literals: Arena<Ast, Option<Literal>>,
}
pub type AstId = Id<Ast>;

// impl Ast {
//     fn pretty_print_indented(&self, id: AstId, indent: usize, files: &Files) {
//         let index_str = id.index().to_string();
//         print!(
//             "{} ",
//             format!(
//                 "[{}{}]",
//                 " ".repeat(3_usize.saturating_sub(index_str.len())),
//                 id.index()
//             )
//             .bright_black()
//         );
//         let kind_str = format!("{:?}", id.get(&self.kinds));
//         let mut len = indent * 2 + kind_str.len();
//         print!("{}{} ", "  ".repeat(indent), kind_str.bold().magenta());
//         let location = id.get(&self.locations);
//         let qualified = id.get(&self.qualified_idents);
//         if !qualified.is_empty() {
//             len += qualified.len() + 1;
//             print!("{} ", qualified.green());
//         } else if let Some(location) = location
//             && id.get(&self.kinds).has_atomic_data()
//         {
//             let slice = &location.file.get(&files.sources)
//                 [location.start as usize..location.end as usize]
//                 .trim();
//             len += slice.len() + 1;
//             print!("{} ", slice);
//         }
//         print!("{} ", " ".repeat(36_usize.saturating_sub(len)));
//         Location::pretty_print_opt(location, files);
//         for child in id.get(&self.children) {
//             self.pretty_print_indented(*child, indent + 1, files);
//         }
//     }
//
//     pub fn pretty_print(&mut self, id: AstId, files: &Files) {
//         println!();
//         self.compute_locations(id);
//         self.pretty_print_indented(id, 0, files);
//     }
// }
