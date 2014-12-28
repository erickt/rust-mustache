use std::collections::HashMap;
use serialize;

use data::Data;
use error::Error;

pub struct Encoder<'a> {
    pub data: Vec<Data<'a>>,
}

impl<'a> Encoder<'a> {
    pub fn new() -> Encoder<'a> {
        Encoder { data: Vec::new() }
    }
}

pub type EncoderResult = Result<(), Error>;

impl<'a> serialize::Encoder<Error> for Encoder<'a> {
    fn emit_nil(&mut self) -> EncoderResult { Err(Error::UnsupportedType) }

    fn emit_uint(&mut self, v: uint) -> EncoderResult { self.emit_str(v.to_string().as_slice()) }
    fn emit_u64(&mut self, v: u64) -> EncoderResult   { self.emit_str(v.to_string().as_slice()) }
    fn emit_u32(&mut self, v: u32) -> EncoderResult   { self.emit_str(v.to_string().as_slice()) }
    fn emit_u16(&mut self, v: u16) -> EncoderResult   { self.emit_str(v.to_string().as_slice()) }
    fn emit_u8(&mut self, v: u8) -> EncoderResult     { self.emit_str(v.to_string().as_slice()) }

    fn emit_int(&mut self, v: int) -> EncoderResult { self.emit_str(v.to_string().as_slice()) }
    fn emit_i64(&mut self, v: i64) -> EncoderResult { self.emit_str(v.to_string().as_slice()) }
    fn emit_i32(&mut self, v: i32) -> EncoderResult { self.emit_str(v.to_string().as_slice()) }
    fn emit_i16(&mut self, v: i16) -> EncoderResult { self.emit_str(v.to_string().as_slice()) }
    fn emit_i8(&mut self, v: i8) -> EncoderResult   { self.emit_str(v.to_string().as_slice()) }

    fn emit_bool(&mut self, v: bool) -> EncoderResult { self.data.push(Data::Bool(v)); Ok(()) }

    fn emit_f64(&mut self, v: f64) -> EncoderResult { self.emit_str(v.to_string().as_slice()) }
    fn emit_f32(&mut self, v: f32) -> EncoderResult { self.emit_str(v.to_string().as_slice()) }

    fn emit_char(&mut self, v: char) -> EncoderResult {
        self.data.push(Data::Str(String::from_char(1, v)));
        Ok(())
    }
    fn emit_str(&mut self, v: &str) -> EncoderResult { self.data.push(Data::Str(v.to_string())); Ok(()) }

    fn emit_enum(&mut self, _name: &str, _f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        Err(Error::UnsupportedType)
    }

    fn emit_enum_variant(&mut self,
                         _name: &str,
                         _id: uint,
                         _len: uint,
                         _f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        Err(Error::UnsupportedType)
    }

    fn emit_enum_variant_arg(&mut self,
                             _a_idx: uint,
                             _f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        Err(Error::UnsupportedType)
    }

    fn emit_enum_struct_variant(&mut self,
                                _v_name: &str,
                                _v_id: uint,
                                _len: uint,
                                _f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        Err(Error::UnsupportedType)
    }

    fn emit_enum_struct_variant_field(&mut self,
                                      _f_name: &str,
                                      _f_idx: uint,
                                      _f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        Err(Error::UnsupportedType)
    }

    fn emit_struct(&mut self,
                   _name: &str,
                   _len: uint,
                   f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        self.data.push(Data::Map(HashMap::new()));
        f(self)
    }

    fn emit_struct_field(&mut self,
                         name: &str,
                         _idx: uint,
                         f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        let mut m = match self.data.pop() {
            Some(Data::Map(m)) => m,
            _ => { return Err(Error::UnsupportedType); }
        };
        try!(f(self));
        let data = match self.data.pop() {
            Some(d) => d,
            _ => { return Err(Error::UnsupportedType); }
        };
        m.insert(name.to_string(), data);
        self.data.push(Data::Map(m));
        Ok(())
    }

    fn emit_tuple(&mut self,
                  len: uint,
                  f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        self.emit_seq(len, f)
    }

    fn emit_tuple_arg(&mut self, idx: uint, f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        self.emit_seq_elt(idx, f)
    }

    fn emit_tuple_struct(&mut self,
                         _name: &str,
                         len: uint,
                         f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        self.emit_seq(len, f)
    }

    fn emit_tuple_struct_arg(&mut self, idx: uint, f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        self.emit_seq_elt(idx, f)
    }

    // Specialized types:
    fn emit_option(&mut self, _f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        Err(Error::UnsupportedType)
    }

    fn emit_option_none(&mut self) -> EncoderResult {
        Err(Error::UnsupportedType)
    }

    fn emit_option_some(&mut self, _f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        Err(Error::UnsupportedType)
    }

    fn emit_seq(&mut self, _len: uint, f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        self.data.push(Data::Vec(Vec::new()));
        f(self)
    }

    fn emit_seq_elt(&mut self, _idx: uint, f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        let mut v = match self.data.pop() {
            Some(Data::Vec(v)) => v,
            _ => { return Err(Error::UnsupportedType); }
        };
        try!(f(self));
        let data = match self.data.pop() {
            Some(d) => d,
            _ => { return Err(Error::UnsupportedType); }
        };
        v.push(data);
        self.data.push(Data::Vec(v));
        Ok(())
    }

    fn emit_map(&mut self, _len: uint, f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        self.data.push(Data::Map(HashMap::new()));
        f(self)
    }

    fn emit_map_elt_key(&mut self, _idx: uint, f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        try!(f(self));
        let last = match self.data.last() {
            Some(d) => d,
            None => { return Err(Error::MissingElements); }
        };
        match *last {
            Data::Str(_) => Ok(()),
            _ => Err(Error::KeyIsNotString),
        }
    }

    fn emit_map_elt_val(&mut self, _idx: uint, f: |&mut Encoder<'a>| -> EncoderResult) -> EncoderResult {
        let k = match self.data.pop() {
            Some(Data::Str(s)) => s,
            _ => { return Err(Error::KeyIsNotString); }
        };
        let mut m = match self.data.pop() {
            Some(Data::Map(m)) => m,
            _ => panic!("Expected a map"),
        };
        try!(f(self));
        let popped = match self.data.pop() {
            Some(p) => p,
            None => panic!("Error: Nothing to pop!"),
        };
        m.insert(k, popped);
        self.data.push(Data::Map(m));
        Ok(())
    }
}

pub fn encode<'a, T: serialize::Encodable<Encoder<'a>, Error>>(data: &T) -> Result<Data<'a>, Error> {
    let mut encoder = Encoder::new();
    try!(data.encode(&mut encoder));
    assert_eq!(encoder.data.len(), 1);
    match encoder.data.pop() {
        Some(data) => Ok(data),
        None => panic!("Error: Nothing to pop!"),
    }
}
