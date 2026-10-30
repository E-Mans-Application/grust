//! [Structure] module.

prelude! {}

/// A structure definition.
#[derive(Debug, PartialEq)]
pub struct Structure {
    /// The structure's name.
    pub name: Ident,
    /// The structure's fields.
    pub fields: Vec<(Ident, Typ)>,
}

mk_new! { impl Structure =>
    new {
        name: impl Into<Ident> = name.into(),
        fields: Vec<(Ident, Typ)>,
    }
}

pub struct StructureTokens<'a> {
    s: &'a Structure,
    public: bool,
    greusot: bool,
}
impl Structure {
    pub fn prepare_tokens(&self, public: bool, greusot: bool) -> StructureTokens<'_> {
        StructureTokens {
            s: self,
            public,
            greusot,
        }
    }
}

impl ToTokens for StructureTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        if self.greusot {
            quote!(
                #[derive(prelude::Clone, Copy, prelude::PartialEq, DeepModel)]
            )
            .to_tokens(tokens)
        } else {
            quote!(#[derive(Clone, Copy, PartialEq, ::grust::core::smart_default::SmartDefault, Debug)]).to_tokens(tokens)
        };
        let pub_token = if self.public {
            quote! {pub}
        } else {
            quote! {}
        };
        let fields = self.s.fields.iter().map(|(name, typ)| {
            let default_value = typ.default_value();
            quote! { #[default(#default_value)] #pub_token #name: #typ }
        });
        let name = &self.s.name;
        quote! {
            #pub_token struct #name {
                #(#fields),*
            }
        }
        .to_tokens(tokens)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_create_rust_ast_structure_from_ir2_structure() {
        let structure = Structure::new(
            Loc::test_id("Point"),
            vec![
                (Loc::test_id("x"), Typ::int()),
                (Loc::test_id("y"), Typ::int()),
            ],
        )
        .prepare_tokens(true, false)
        .to_token_stream();

        let control = parse_quote! {
            #[derive(Clone, Copy, PartialEq, ::grust::core::smart_default::SmartDefault, Debug)]
            pub struct Point {
                #[default(::core::default::Default::default())]
                pub x: i64,
                #[default(::core::default::Default::default())]
                pub y: i64
            }
        };
        let structure: syn::ItemStruct = parse_quote!(#structure);
        assert_eq!(structure, control)
    }
}
