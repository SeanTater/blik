pub(crate) trait MaybeString {
    fn str_or<'t>(&'t self, default: &'t str) -> &'t str;
}

impl MaybeString for Option<String> {
    fn str_or<'t>(&'t self, default: &'t str) -> &'t str {
        match self {
            Some(ref content) => content,
            None => default
        }
    }
}