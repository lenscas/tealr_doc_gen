# tealr_doc_gen

This tool is meant to be used together with [tealr](https://github.com/lenscas/tealr/tree/master/tealr) and is used to generate online documentation for lua/teal apis created with [tealr](https://github.com/lenscas/tealr/tree/master/tealr)

## Rendered Example 
https://lenscas.github.io/tealsql/

## Features:
 - full markdown support
 - code highlighting in code examples
 - multiple theme support
 - snippets marked as `teal_lua` get compiled to `lua` and both versions get embedded.
 - When compiling `teal_lua` snippets, any errors get logged.

# How to get the json
The json file needed to generate the documentation can easily be gotten using
```rs
use tealr::{
    TypeWalker,
};

fn main() {
    let types = TypeWalker::new()
        .process_type::<crate::TypeYouWantToDocument>()
        .process_type<crate::OtherTypeYouWantToDocument>();
    
    let json = serde_json::to_string_pretty(&types).unwrap();
    println!("{}",json); //save to a file
    
}
```
# Install

simply run `cargo install tealr_doc_gen` to install. Cargo will do the rest.

After it is installed you can run `tealr_doc_gen --json path/to/json/file --name yourApiName` to generate the documentation.

# Arguments

## Required arguments
-  `--json` `/path/to/json/generated/by/tealr`
- `--name` `nameOfTheLibrary`
## Optional arguments
- `--build_folder` What folder to store the generated html pages at (defaults to `./pages`)
- `--root` set if `/` will not be the root of the server

# How to get the json
The json file needed to generate the documentation can easily be gotten using
```rs
use tealr::{
    TypeWalker,
};

fn main() {
    let types = TypeWalker::new()
        .process_type::<crate::TypeYouWantToDocument>()
        .process_type<crate::OtherTypeYouWantToDocument>();
    
    let json = serde_json::to_string_pretty(&types).unwrap();
    println!("{}",json); //save to a file
    
}
```