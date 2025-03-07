use crate::idl;

#[derive(Default)]
pub struct Writer {
    out: String,
    indent: usize,
    newline: bool,
}

impl Writer {
    pub fn new(file: &idl::File) -> Self {
        let mut writer = Self::default();
        writer.idl_file(file);
        writer
    }

    pub fn into_string(self) -> String {
        self.out
    }

    fn word(&mut self, value: &str) {
        if self.newline {
            self.newline = false;
            self.out.push('\n');
            for _ in 0..self.indent {
                self.out.push_str("    ");
            }
        }

        self.out.push_str(value);
    }

    fn newline(&mut self) {
        self.newline = true;
    }

    fn idl_file(&mut self, file: &idl::File) {
        for reference in &file.references {
            self.item_use(reference);
        }

        for module in &file.modules {
            self.idl_module(module);
        }
    }

    fn idl_module(&mut self, module: &idl::Module) {
        self.word("mod ");
        self.word(&module.name);
        self.word(" {");
        self.newline();
        self.indent += 1;

        for member in &module.members {
            self.idl_module_member(member);
            self.newline();
        }

        self.indent -= 1;
        self.newline();
        self.word("}");
    }

    fn idl_module_member(&mut self, member: &idl::ModuleMember) {
        match member {
            idl::ModuleMember::Module(member) => self.idl_module(member),
            idl::ModuleMember::Interface(member) => self.idl_interface(member),
            idl::ModuleMember::Struct(member) => self.idl_struct(member),
            idl::ModuleMember::Enum(member) => self.idl_enum(member),
            idl::ModuleMember::Class(member) => self.idl_class(member),
        }
    }

    fn idl_interface(&mut self, member: &idl::Interface) {
        self.attrs(&member.attributes);
        self.word("interface ");
        self.word(&member.name);
        self.word(" {");
        self.newline();
        self.indent += 1;

        for method in &member.methods {
            self.trait_item_fn(method);
            self.word(";");
            self.newline();
        }

        self.indent -= 1;
        self.newline();
        self.word("}");
    }

    fn attrs(&mut self, attrs: &[syn::Attribute]) {
        for attr in attrs {
            self.attr(attr);
        }
    }

    fn attr(&mut self, attr: &syn::Attribute) {
        self.word("#[");
        self.meta(&attr.meta);
        self.word("]");
        self.newline();
    }

    fn meta(&mut self, meta: &syn::Meta) {
        match meta {
            syn::Meta::Path(path) => self.path(path),
            syn::Meta::List(list) => self.meta_list(list),
            rest => unimplemented!("{rest:?}"),
        }
    }

    fn meta_list(&mut self, meta_list: &syn::MetaList) {
        self.path(&meta_list.path);
        self.word("(");
        self.word(&meta_list.tokens.to_string());
        self.word(")");
    }

    fn idl_struct(&mut self, member: &idl::Struct) {
        self.attrs(&member.attributes);

        self.word("struct ");
        self.word(&member.name);
        self.word(" {");
        self.newline();
        self.indent += 1;

        for field in &member.fields {
            self.word(&field.name);
            self.word(": ");
            self.ty(&field.ty);
            self.word(",");
            self.newline();
        }

        self.indent -= 1;
        self.newline();
        self.word("}");
    }

    fn idl_enum(&mut self, member: &idl::Enum) {
        self.attrs(&member.item.attrs);

        self.word("enum ");
        self.ident(&member.item.ident);
        self.word(" {");
        self.newline();
        self.indent += 1;

        for variant in &member.item.variants {
            self.ident(&variant.ident);
            if let Some((_, expr)) = &variant.discriminant {
                self.word(" = ");
                self.expr(expr);
            }
            self.word(",");
            self.newline();
        }

        self.indent -= 1;
        self.newline();
        self.word("}");
    }

    fn idl_class(&mut self, _member: &idl::Class) {}

    fn trait_item_fn(&mut self, method: &syn::TraitItemFn) {
        self.attrs(&method.attrs);
        self.signature(&method.sig);
    }

    fn signature(&mut self, signature: &syn::Signature) {
        self.word("fn ");
        self.ident(&signature.ident);
        self.word("(");

        let mut first = true;
        for input in &signature.inputs {
            if first {
                first = false;
            } else {
                self.word(", ");
            }
            self.fn_arg(input);
        }

        self.word(")");

        if let syn::ReturnType::Type(_, ty) = &signature.output {
            self.word(" -> ");
            self.ty(ty);
        }
    }

    fn fn_arg(&mut self, fn_arg: &syn::FnArg) {
        if let syn::FnArg::Typed(pat_type) = fn_arg {
            self.pat_type(pat_type);
        }
    }

    fn pat_type(&mut self, pat_type: &syn::PatType) {
        self.pat(&pat_type.pat);
        self.word(": ");
        self.ty(&pat_type.ty);
    }

    fn pat(&mut self, pat: &syn::Pat) {
        match pat {
            syn::Pat::Ident(pat_ident) => self.pat_ident(pat_ident),
            rest => unimplemented!("{rest:?}"),
        }
    }

    fn pat_ident(&mut self, pat_ident: &syn::PatIdent) {
        self.ident(&pat_ident.ident);
    }

    fn ty(&mut self, ty: &syn::Type) {
        match ty {
            syn::Type::Path(ty) => self.type_path(ty),
            syn::Type::Ptr(ptr) => self.type_ptr(ptr),
            syn::Type::Array(array) => self.type_array(array),
            rest => unimplemented!("{rest:?}"),
        }
    }

    fn type_array(&mut self, array: &syn::TypeArray) {
        self.word("[");
        self.ty(&array.elem);
        self.word("; ");
        self.expr(&array.len);
        self.word("]");
    }

    fn expr(&mut self, expr: &syn::Expr) {
        match expr {
            syn::Expr::Lit(lit) => self.expr_lit(lit),
            syn::Expr::Unary(unary) => self.expr_unary(unary),
            rest => unimplemented!("{rest:?}"),
        }
    }

    fn expr_unary(&mut self, unary: &syn::ExprUnary) {
        self.word("-");
        self.expr(&unary.expr);
    }

    fn expr_lit(&mut self, expr: &syn::ExprLit) {
        self.lit(&expr.lit);
    }

    fn lit(&mut self, lit: &syn::Lit) {
        match lit {
            syn::Lit::Int(lit) => self.lit_int(lit),
            syn::Lit::Str(lit) => self.lit_str(lit),
            _ => _ = dbg!(lit),
        }
    }

    fn lit_str(&mut self, lit: &syn::LitStr) {
        self.word("\"");
        self.word(&lit.value());
        self.word("\"");
    }

    fn lit_int(&mut self, lit: &syn::LitInt) {
        self.word(&lit.token().to_string());
    }

    fn type_ptr(&mut self, ptr: &syn::TypePtr) {
        if ptr.mutability.is_some() {
            self.word("*mut ");
        } else {
            self.word("*const ");
        }
        self.ty(&ptr.elem);
    }

    fn type_path(&mut self, ty: &syn::TypePath) {
        self.path(&ty.path);
    }

    fn path(&mut self, path: &syn::Path) {
        let mut first = true;
        for segment in &path.segments {
            if first {
                first = false;
            } else {
                self.word("::");
            }
            self.path_segment(segment);
        }
    }

    pub fn path_segment(&mut self, segment: &syn::PathSegment) {
        self.ident(&segment.ident);
    }

    fn item_use(&mut self, item: &syn::ItemUse) {
        self.word("use ");
        self.use_tree(&item.tree);
        self.word(";");
        self.newline();
    }

    fn use_tree(&mut self, use_tree: &syn::UseTree) {
        match use_tree {
            syn::UseTree::Path(use_path) => self.use_path(use_path),
            syn::UseTree::Name(use_name) => self.use_name(use_name),
            _ => {}
        }
    }

    fn use_path(&mut self, use_path: &syn::UsePath) {
        self.ident(&use_path.ident);
        self.word("::");
        self.use_tree(&use_path.tree);
    }

    fn use_name(&mut self, use_name: &syn::UseName) {
        self.ident(&use_name.ident);
    }

    pub fn ident(&mut self, ident: &syn::Ident) {
        self.word(&ident.to_string());
    }
}
