use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

pub enum Data<'a> {
    Str(String),
    Bool(bool),
    Vec(Vec<Data<'a>>),
    Map(HashMap<String, Data<'a>>),
    Fun(RefCell<|String|: 'a -> String>),
}

impl<'a> PartialEq for Data<'a> {
    #[inline]
    fn eq(&self, other: &Data<'a>) -> bool {
        match (self, other) {
            (&Data::Str(ref v0), &Data::Str(ref v1)) => v0 == v1,
            (&Data::Bool(ref v0), &Data::Bool(ref v1)) => v0 == v1,
            (&Data::Vec(ref v0), &Data::Vec(ref v1)) => v0 == v1,
            (&Data::Map(ref v0), &Data::Map(ref v1)) => v0 == v1,
            (&Data::Fun(_), &Data::Fun(_)) => panic!("cannot compare closures"),
            (_, _) => false,
        }
    }
}

impl<'a> fmt::Show for Data<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Data::Str(ref v) => write!(f, "Str({})", v),
            Data::Bool(v) => write!(f, "Bool({})", v),
            Data::Vec(ref v) => write!(f, "Vec({})", v),
            Data::Map(ref v) => write!(f, "Map({})", v),
            Data::Fun(_) => write!(f, "Fun(...)"),
        }
    }
}
