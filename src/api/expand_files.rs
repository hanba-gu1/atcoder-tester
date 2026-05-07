use std::{fs, path::Path};

use anyhow::{Result, anyhow};
use syn_file_expand::read_full_crate_source_code;

pub fn expand_files(main_crate: &Path, library_crate: &Path) -> Result<String> {
    let main_file = fs::read_to_string(main_crate)?;

    let libs_file = read_full_crate_source_code(library_crate, |_| Ok(false))
        .map_err(|err| anyhow!("{err}"))?;

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
