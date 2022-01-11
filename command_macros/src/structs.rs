use std::ops::DerefMut;

use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    quote,
    ToTokens,
};
use syn::Lifetime;
use syn::parse::{
    Parse,
    ParseStream,
    Result,
};
use syn::{
    braced,
    Attribute,
    Block,
    FnArg,
    Ident,
    Path,
    PathSegment,
    ReturnType,
    Stmt,
    Token,
    Type,
    Visibility,
};

use super::utils::ParenthesisedItems;

#[derive(Debug)]
pub struct CommandFunc {
    /// `#[...]`-style attributes.
    pub attributes: Vec<Attribute>,
    /// Populated by `#[cfg(...)]` type attributes.
    pub cooked: Vec<Attribute>,
    pub visibility: Visibility,
    pub name: Ident,
    pub ret: Type,
    pub args: Vec<FnArg>,
    pub body: Vec<Stmt>,
}

impl Parse for CommandFunc {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attributes = input.call(Attribute::parse_outer)?;

        let (cooked, attributes): (Vec<_>, Vec<_>) = attributes
            .into_iter()
            .map(|mut a| {
                if a.path
                    .is_ident("doc")
                {
                    a.path = Path::from(PathSegment::from(Ident::new(
                        "description",
                        Span::call_site(),
                    )));
                    let ts: TokenStream2 = a.tokens.into_iter().skip(1).collect();
                    a.tokens = quote! {(#ts)};
                    a
                } else {
                    a
                }
            })
            .partition(|a| {
                a.path
                    .is_ident("cfg")
            });

        let visibility = input.parse::<Visibility>()?;

        input.parse::<Token![async]>()?;
        input.parse::<Token![fn]>()?;
        let name = input.parse()?;

        let ParenthesisedItems(mut args) = input.parse::<ParenthesisedItems<FnArg>>()?;
        for arg in args.iter_mut() {
            if let FnArg::Typed(tped) = arg {
                if let Type::Reference(r) = tped.ty.deref_mut() {
                    r.lifetime = Some(Lifetime::new("'fut", Span::call_site()))
                }
            }
        }

        let ret = match input.parse::<ReturnType>()? {
            ReturnType::Type(_, t) => *t,
            ReturnType::Default => return Err(input.error("expected a CommandResult return value")),
        };

        let body_content;
        braced!(body_content in input);
        let body: Vec<Stmt> = body_content.call(Block::parse_within)?;

        let args = args
            .into_iter()
            .collect::<Vec<FnArg>>();

        Ok(Self {
            attributes,
            cooked,
            visibility,
            name,
            ret,
            args,
            body,
        })
    }
}

impl ToTokens for CommandFunc {
    fn to_tokens(&self, stream: &mut TokenStream2) {
        let Self {
            attributes: _,
            cooked,
            visibility,
            name,
            args,
            ret,
            body,
        } = self;

        stream.extend(quote! {
            #(#cooked)*
            #visibility fn #name <'fut> (#(#args),*) -> futures::future::BoxFuture<'fut, #ret> {
                ::std::boxed::Box::pin(async move {
                    #(#body)*
                })
            }
        });
    }
}
