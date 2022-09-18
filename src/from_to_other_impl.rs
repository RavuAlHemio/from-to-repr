use proc_macro2::TokenTree;
use syn::{Path, Token};
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use syn::punctuated::Punctuated;


pub(crate) struct KeyValuePair {
    pub path: Path,
    pub eq_token: Token![=],
    pub token_tree: TokenTree,
}
impl Parse for KeyValuePair {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let path = input.parse()?;
        let eq_token = input.parse()?;
        let token_tree = input.parse()?;
        Ok(Self {
            path,
            eq_token,
            token_tree,
        })
    }
}


pub(crate) struct KeyValuePairs {
    pub kvps: Vec<KeyValuePair>,
}
impl Parse for KeyValuePairs {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let args_parsed = Punctuated::<KeyValuePair, Token![,]>::parse_terminated(input)?;
        let kvps = args_parsed.into_iter().collect();
        Ok(Self {
            kvps,
        })
    }
}
