use std::{fs, path::Path};

use anyhow::{Result, anyhow};
use syn::{Attribute, ImplItem, Item, TraitItem, visit_mut::VisitMut};
use syn_file_expand::read_full_crate_source_code;

fn contains_removed_cfg(attrs: &[Attribute]) -> bool {
    let removed_attrs = ["test", "debug_assertions"];

    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Ident>()
                .is_ok_and(|arg| removed_attrs.iter().any(|&ra| arg == ra))
    })
}

macro_rules! get_attrs {
    ($arg:expr, $($var:ident),* $(,)?) => {
        match $arg {
            $($var(e) => &e.attrs,)*
            _ => &[],
        }
    };
}

fn get_expr_attrs(expr: &syn::Expr) -> &[Attribute] {
    use syn::Expr::*;
    get_attrs!(
        expr, Array, Assign, Async, Await, Binary, Block, Break, Call, Cast, Closure, Const,
        Continue, Field, ForLoop, Group, If, Index, Infer, Let, Lit, Loop, Macro, Match,
        MethodCall, Paren, Path, Range, Reference, Repeat, Return, Struct, Try, TryBlock, Tuple,
        Unary, Unsafe, While, Yield,
    )
}

fn get_stmt_attrs(stmt: &syn::Stmt) -> &[Attribute] {
    use syn::Stmt::*;
    match stmt {
        Local(l) => &l.attrs,
        Item(i) => get_item_attrs(i),
        Expr(e, _) => get_expr_attrs(e),
        Macro(m) => &m.attrs,
    }
}

#[rustfmt::skip]
fn get_item_attrs(item: &Item) -> &[Attribute] {
    use Item::*;
    get_attrs!(
        item, Const, Enum, ExternCrate, Fn, ForeignMod, Impl, Macro,
        Mod, Static, Struct, Trait, TraitAlias, Type, Union, Use,
    )
}

fn get_impl_item_attrs(item: &ImplItem) -> &[Attribute] {
    use ImplItem::*;
    get_attrs!(item, Const, Fn, Type, Macro)
}

fn get_trait_item_attrs(item: &TraitItem) -> &[Attribute] {
    use TraitItem::*;
    get_attrs!(item, Const, Fn, Type, Macro)
}

fn retain_punctuated<T, P, F>(punct: &mut syn::punctuated::Punctuated<T, P>, mut keep: F)
where
    F: FnMut(&T) -> bool,
{
    let old_punct = std::mem::take(punct);
    for pair in old_punct.into_pairs() {
        let (value, punct_token) = pair.into_tuple();
        if keep(&value) {
            punct.push_value(value);
            if let Some(pt) = punct_token {
                punct.push_punct(pt);
            }
        }
    }
}

struct DebugCfgsRemover;

impl VisitMut for DebugCfgsRemover {
    fn visit_file_mut(&mut self, node: &mut syn::File) {
        node.items
            .retain(|item| !contains_removed_cfg(get_item_attrs(item)));
        syn::visit_mut::visit_file_mut(self, node);
    }

    fn visit_item_mod_mut(&mut self, node: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut node.content {
            items.retain(|item| !contains_removed_cfg(get_item_attrs(item)));
        }
        syn::visit_mut::visit_item_mod_mut(self, node);
    }

    fn visit_block_mut(&mut self, node: &mut syn::Block) {
        node.stmts
            .retain(|stmt| !contains_removed_cfg(get_stmt_attrs(stmt)));
        syn::visit_mut::visit_block_mut(self, node);
    }

    fn visit_item_struct_mut(&mut self, node: &mut syn::ItemStruct) {
        match &mut node.fields {
            syn::Fields::Named(f) => {
                retain_punctuated(&mut f.named, |field| !contains_removed_cfg(&field.attrs))
            }
            syn::Fields::Unnamed(f) => {
                retain_punctuated(&mut f.unnamed, |field| !contains_removed_cfg(&field.attrs))
            }
            syn::Fields::Unit => {}
        }
        syn::visit_mut::visit_item_struct_mut(self, node);
    }

    fn visit_item_enum_mut(&mut self, node: &mut syn::ItemEnum) {
        retain_punctuated(&mut node.variants, |variant| {
            !contains_removed_cfg(&variant.attrs)
        });
        syn::visit_mut::visit_item_enum_mut(self, node);
    }

    fn visit_signature_mut(&mut self, node: &mut syn::Signature) {
        retain_punctuated(&mut node.inputs, |arg| match arg {
            syn::FnArg::Receiver(r) => !contains_removed_cfg(&r.attrs),
            syn::FnArg::Typed(t) => !contains_removed_cfg(&t.attrs),
        });
        syn::visit_mut::visit_signature_mut(self, node);
    }

    fn visit_expr_match_mut(&mut self, node: &mut syn::ExprMatch) {
        node.arms.retain(|arm| !contains_removed_cfg(&arm.attrs));
        syn::visit_mut::visit_expr_match_mut(self, node);
    }

    fn visit_expr_array_mut(&mut self, node: &mut syn::ExprArray) {
        retain_punctuated(&mut node.elems, |expr| {
            !contains_removed_cfg(get_expr_attrs(expr))
        });
        syn::visit_mut::visit_expr_array_mut(self, node);
    }

    fn visit_expr_call_mut(&mut self, node: &mut syn::ExprCall) {
        retain_punctuated(&mut node.args, |expr| {
            !contains_removed_cfg(get_expr_attrs(expr))
        });
        syn::visit_mut::visit_expr_call_mut(self, node);
    }

    fn visit_expr_try_block_mut(&mut self, node: &mut syn::ExprTryBlock) {
        node.block
            .stmts
            .retain(|stmt| !contains_removed_cfg(get_stmt_attrs(stmt)));
        syn::visit_mut::visit_expr_try_block_mut(self, node);
    }

    fn visit_item_impl_mut(&mut self, node: &mut syn::ItemImpl) {
        node.items
            .retain(|item| !contains_removed_cfg(get_impl_item_attrs(item)));
        syn::visit_mut::visit_item_impl_mut(self, node);
    }

    fn visit_item_trait_mut(&mut self, node: &mut syn::ItemTrait) {
        node.items
            .retain(|item| !contains_removed_cfg(get_trait_item_attrs(item)));
        syn::visit_mut::visit_item_trait_mut(self, node);
    }
}

pub fn expand_files(main_crate: &Path, library_crate: &Path) -> Result<String> {
    let main_file = fs::read_to_string(main_crate)?;

    let mut libs_file = read_full_crate_source_code(library_crate, |_| Ok(false))
        .map_err(|err| anyhow!("{err}"))?;

    DebugCfgsRemover.visit_file_mut(&mut libs_file);

    let syn::File { attrs, items, .. } = libs_file;

    let libs_file: syn::File = syn::parse_quote! {
        #[allow(unused)]
        mod libs {
            #(#attrs)*
            #(#items)*
        }
    };

    let result_file = format!("{main_file}\n{}", prettyplease::unparse(&libs_file));

    Ok(result_file)
}
