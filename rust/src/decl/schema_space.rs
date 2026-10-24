use crate::decl::DeclProv;

pub struct SchemeSpace<'prov> {
	prov: &'prov dyn DeclProv,
}
impl SchemeSpace<'_> {}
