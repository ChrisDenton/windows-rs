use crate::tokens::{quote, to_ident, TokenStream};
use crate::{idl, Error, Result, Tree};

pub fn from_reader(
    reader: &metadata::Reader,
    filter: &metadata::Filter,
    config: std::collections::BTreeMap<&str, &str>,
    output: &str,
) -> Result<()> {
    let writer = Writer::new(reader, filter);

    // TODO: do we need any configuration values for IDL generation?
    // Maybe per-namespace IDL files for namespace-splitting - be sure to use
    // the same key as for winmd generation.

    if let Some((key, _)) = config.first_key_value() {
        return Err(Error::new(&format!("invalid configuration value: `{key}`")));
    }

    let tree = Tree::new(writer.reader, writer.filter);
    let tokens = writer.tree(&tree);
    let file = idl::File::parse_str(&tokens.into_string())?;
    crate::write_to_file(output, file.fmt())
}

struct Writer<'a> {
    reader: &'a metadata::Reader<'a>,
    filter: &'a metadata::Filter<'a>,
    namespace: &'a str,
}

impl<'a> Writer<'a> {
    fn new(reader: &'a metadata::Reader, filter: &'a metadata::Filter) -> Self {
        Self {
            reader,
            filter,
            namespace: "",
        }
    }

    fn with_namespace(&self, namespace: &'a str) -> Self {
        Self {
            reader: self.reader,
            filter: self.filter,
            namespace,
        }
    }

    fn tree(&self, tree: &'a Tree) -> TokenStream {
        let modules = tree
            .nested
            .values()
            .map(|tree| self.with_namespace(tree.namespace).tree(tree));

        if tree.namespace.is_empty() {
            quote! { #(#modules)* }
        } else {
            let name = to_ident(
                tree.namespace
                    .rsplit_once('.')
                    .map_or(tree.namespace, |(_, name)| name),
            );
            let types = self
                .reader
                .namespace_types(tree.namespace, self.filter)
                .map(|def| self.type_def(def));

            quote! {
                mod #name {
                    #(#modules)*
                    #(#types)*
                }
            }
        }
    }

    fn type_def(&self, def: metadata::TypeDef) -> TokenStream {
        if let Some(extends) = self.reader.type_def_extends(def) {
            if extends.namespace == "System" {
                if extends.name == "Enum" {
                    self.enum_def(def)
                } else if extends.name == "ValueType" {
                    self.struct_def(def)
                } else if extends.name == "MulticastDelegate" {
                    self.delegate_def(def)
                } else {
                    self.class_def(def)
                }
            } else {
                self.class_def(def)
            }
        } else {
            self.interface_def(def)
        }
    }

    fn enum_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(self.reader.type_def_name(def));

        quote! {
            struct #name {

            }
        }
    }

    fn struct_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(self.reader.type_def_name(def));

        let fields = self.reader.type_def_fields(def).map(|field| {
            let name = to_ident(self.reader.field_name(field));
            let ty = self.ty(&self.reader.field_type(field, Some(def)));
            quote! {
                #name: #ty
            }
        });

        quote! {
            struct #name {
                #(#fields),*
            }
        }
    }

    fn delegate_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(self.reader.type_def_name(def));

        quote! {
            struct #name {

            }
        }
    }

    fn class_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(self.reader.type_def_name(def));

        quote! {
            struct #name {

            }
        }
    }

    fn interface_def(&self, def: metadata::TypeDef) -> TokenStream {
        let name = to_ident(self.reader.type_def_name(def));

        quote! {
            struct #name {

            }
        }
    }

    fn ty(&self, ty: &metadata::Type) -> TokenStream {
        match ty {
            metadata::Type::Void => quote! { ::core::ffi::c_void },
            metadata::Type::Bool => quote! { bool },
            metadata::Type::Char => quote! { u16 },
            metadata::Type::I8 => quote! { i8 },
            metadata::Type::U8 => quote! { u8 },
            metadata::Type::I16 => quote! { i16 },
            metadata::Type::U16 => quote! { u16 },
            metadata::Type::I32 => quote! { i32 },
            metadata::Type::U32 => quote! { u32 },
            metadata::Type::I64 => quote! { i64 },
            metadata::Type::U64 => quote! { u64 },
            metadata::Type::F32 => quote! { f32 },
            metadata::Type::F64 => quote! { f64 },
            metadata::Type::ISize => quote! { isize },
            metadata::Type::USize => quote! { usize },
            metadata::Type::TypeDef(def, generics) => {
                let namespace = self.namespace(self.reader.type_def_namespace(*def));
                let name = to_ident(self.reader.type_def_name(*def));
                if generics.is_empty() {
                    quote! { #namespace#name }
                } else {
                    let generics = generics.iter().map(|ty| self.ty(ty));
                    quote! { #namespace#name<#(#generics,)*> }
                }
            }
            rest => unimplemented!("{rest:?}"),
        }
    }

    fn namespace(&self, namespace: &str) -> TokenStream {
        // TODO: handle nested structs?
        if namespace.is_empty() || self.namespace == namespace {
            quote! {}
        } else {
            let mut relative = self.namespace.split('.').peekable();
            let mut namespace = namespace.split('.').peekable();
            let mut related = false;

            while relative.peek() == namespace.peek() {
                related = true;

                if relative.next().is_none() {
                    break;
                }

                namespace.next();
            }

            let mut tokens = TokenStream::new();

            if related {
                for _ in 0..relative.count() {
                    tokens.push_str("super::");
                }
            }

            for namespace in namespace {
                tokens.push_str(namespace);
                tokens.push_str("::");
            }

            tokens
        }
    }
}
