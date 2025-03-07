use super::*;

pub fn gen(gen: &Gen, def: TypeDef) -> TokenStream {
    let type_name = gen.reader.type_def_type_name(def);
    let ident = to_ident(type_name.name);
    let underlying_type = gen.reader.type_def_underlying_type(def);
    let underlying_type = gen.type_name(&underlying_type);
    let is_scoped = gen.reader.type_def_is_scoped(def);
    let cfg = gen.reader.type_def_cfg(def, &[]);
    let doc = gen.cfg_doc(&cfg);
    let features = gen.cfg_features(&cfg);

    let fields: Vec<(TokenStream, TokenStream)> = gen
        .reader
        .type_def_fields(def)
        .filter_map(|field| {
            if gen
                .reader
                .field_flags(field)
                .contains(FieldAttributes::Literal)
            {
                let field_name = to_ident(gen.reader.field_name(field));
                let constant = gen.reader.field_constant(field).unwrap();
                let value = gen.value(&gen.reader.constant_value(constant));

                Some((field_name, value))
            } else {
                None
            }
        })
        .collect();

    let eq = if gen.sys {
        quote! {}
    } else {
        quote! {
            // Unfortunately, Rust requires these to be derived to allow constant patterns.
            #[derive(::core::cmp::PartialEq, ::core::cmp::Eq)]
        }
    };

    let mut tokens = if is_scoped || !gen.sys {
        quote! {
            #doc
            #features
            #[repr(transparent)]
            #eq
            pub struct #ident(pub #underlying_type);
        }
    } else {
        quote! {
            #doc
            #features
            pub type #ident = #underlying_type;
        }
    };

    if is_scoped {
        let fields = fields.iter().map(|(field_name, value)| {
            quote! {
                pub const #field_name: Self = Self(#value);
            }
        });

        tokens.combine(&quote! {
            #features
            impl #ident {
                #(#fields)*
            }
        });
    } else if !gen.minimal {
        if gen.sys {
            let fields = fields.iter().map(|(field_name, value)| {
                quote! {
                    #doc
                    #features
                    pub const #field_name: #ident = #value;
                }
            });

            tokens.combine(&quote! {
                #(#fields)*
            });
        } else {
            let fields = fields.iter().map(|(field_name, value)| {
                quote! {
                    #doc
                    #features
                    pub const #field_name: #ident = #ident(#value);
                }
            });

            tokens.combine(&quote! {
                #(#fields)*
            });
        }
    }

    if is_scoped || !gen.sys {
        tokens.combine(&quote! {
            #features
            impl ::core::marker::Copy for #ident {}
            #features
            impl ::core::clone::Clone for #ident {
                fn clone(&self) -> Self {
                    *self
                }
            }
        });
    }

    if !gen.sys {
        tokens.combine(&quote! {
            #features
            impl ::core::default::Default for #ident {
                fn default() -> Self {
                    Self(0)
                }
            }
        });
    }

    if !gen.sys {
        let name = type_name.name;
        tokens.combine(&quote! {
            #features
            impl ::windows_core::TypeKind for #ident {
                type TypeKind = ::windows_core::CopyType;
            }
            #features
            impl ::core::fmt::Debug for #ident {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    f.debug_tuple(#name).field(&self.0).finish()
                }
            }
        });

        if gen.reader.type_def_is_flags(def) {
            tokens.combine(&quote! {
                #features
                impl #ident {
                    pub const fn contains(&self, other: Self) -> bool {
                        self.0 & other.0 == other.0
                    }
                }
                #features
                impl ::core::ops::BitOr for #ident {
                    type Output = Self;

                    fn bitor(self, other: Self) -> Self {
                        Self(self.0 | other.0)
                    }
                }
                #features
                impl ::core::ops::BitAnd for #ident {
                    type Output = Self;

                    fn bitand(self, other: Self) -> Self {
                        Self(self.0 & other.0)
                    }
                }
                #features
                impl ::core::ops::BitOrAssign for #ident {
                    fn bitor_assign(&mut self, other: Self) {
                        self.0.bitor_assign(other.0)
                    }
                }
                #features
                impl ::core::ops::BitAndAssign for #ident {
                    fn bitand_assign(&mut self, other: Self) {
                        self.0.bitand_assign(other.0)
                    }
                }
                #features
                impl ::core::ops::Not for #ident {
                    type Output = Self;

                    fn not(self) -> Self {
                        Self(self.0.not())
                    }
                }
            });
        }

        if gen
            .reader
            .type_def_flags(def)
            .contains(TypeAttributes::WindowsRuntime)
        {
            let signature =
                Literal::byte_string(gen.reader.type_def_signature(def, &[]).as_bytes());

            tokens.combine(&quote! {
                #features
                impl ::windows_core::RuntimeType for #ident {
                    const SIGNATURE: ::windows_core::imp::ConstBuffer = ::windows_core::imp::ConstBuffer::from_slice(#signature);
                }
            });
        }
    }

    tokens
}
