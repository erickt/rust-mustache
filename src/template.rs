use std::io::Write;
use std::collections::HashMap;
use std::mem;
use std::str;
use rustc_serialize::Encodable;

use encoder;
use compiler::Compiler;
use parser_internals::Token;
use parser_internals::Token::*;

use super::{Context, Data, Bool, StrVal, VecVal, Map, Fun, OptVal, Result};

/// `Template` represents a compiled mustache file.
#[derive(Debug, Clone)]
pub struct Template {
    ctx: Context,
    tokens: Vec<Token>,
    partials: HashMap<String, Vec<Token>>,
}

/// Construct a `Template`. This is not part of the impl of Template so it is
/// not exported outside of mustache.
pub fn new(ctx: Context, tokens: Vec<Token>, partials: HashMap<String, Vec<Token>>) -> Template {
    Template {
        ctx: ctx,
        tokens: tokens,
        partials: partials,
    }
}

impl Template {
    /// Renders the template with the `Encodable` data.
    pub fn render<W: Write, T: Encodable>(&self, wr: &mut W, data: &T) -> Result {
        let data = try!(encoder::encode(data));
        self.render_data(wr, &data)
    }

    /// Renders the template with the `Data`.
    pub fn render_data<W: Write>(&self, wr: &mut W, data: &Data) -> Result {
        let mut render_ctx = RenderContext::new(self);
        let mut stack = vec![data];

        render_ctx.render(wr, &mut stack, &self.tokens)
    }
}

struct RenderContext<'a> {
    template: &'a Template,
    indent: String,
    line_start: bool,
}

impl<'a> RenderContext<'a> {
    fn new(template: &'a Template) -> RenderContext<'a> {
        RenderContext {
            template: template,
            indent: "".to_string(),
            line_start: true,
        }
    }

    fn render<W: Write>(&mut self, wr: &mut W, stack: &mut Vec<&Data>, tokens: &[Token]) -> Result {
        for token in tokens.iter() {
            try!(self.render_token(wr, stack, token));
        }

        Ok(())
    }

    fn render_token<W: Write>(&mut self, wr: &mut W, stack: &mut Vec<&Data>, token: &Token) -> Result {
        match *token {
            Text(ref value) => {
                self.render_text(wr, value)
            }
            EscapedTag(ref path, _) => {
                self.render_etag(wr, stack, path)
            }
            UnescapedTag(ref path, _) => {
                self.render_utag(wr, stack, path)
            }
            Section(ref path, true, ref children, _, _, _, _, _) => {
                self.render_inverted_section(wr, stack, path, children)
            }
            Section(ref path, false, ref children, ref otag, _, ref src, _, ref ctag) => {
                self.render_section(wr, stack, path, children, src, otag, ctag)
            }
            Partial(ref name, ref indent, _) => {
                self.render_partial(wr, stack, name, indent)
            }
            IncompleteSection(..) => {
                bug!("render_token should not encounter IncompleteSections")
            }
        }
    }

    fn write_tracking_newlines<W: Write>(&mut self, wr: &mut W, value: &str) -> Result {
        try!(wr.write_all(value.as_bytes()));
        self.line_start = match value.chars().last() {
            None => self.line_start, // None == ""
            Some('\n') => true,
            _ => false,
        };

        Ok(())
    }

    fn write_indent<W: Write>(&mut self, wr: &mut W) -> Result {
        if self.line_start {
            try!(wr.write_all(self.indent.as_bytes()));
        }

        Ok(())
    }

    fn render_text<W: Write>(&mut self, wr: &mut W, value: &str) -> Result {
        // Indent the lines.
        if self.indent.is_empty() {
            return self.write_tracking_newlines(wr, value);
        } else {
            let mut pos = 0;
            let len = value.len();

            while pos < len {
                let v = &value[pos..];
                let line = match v.find('\n') {
                    None => {
                        let line = v;
                        pos = len;
                        line
                    }
                    Some(i) => {
                        let line = &v[..i + 1];
                        pos += i + 1;
                        line
                    }
                };

                if line.as_bytes()[0] != b'\n' {
                    try!(self.write_indent(wr));
                }

                try!(self.write_tracking_newlines(wr, line));
            }
        }

        Ok(())
    }

    fn render_etag<W: Write>(&mut self, wr: &mut W, stack: &mut Vec<&Data>, path: &[String]) -> Result {
        let mut bytes = vec![];

        try!(self.render_utag(&mut bytes, stack, path));

        for b in bytes {
            match b {
                b'<' => {
                    try!(wr.write_all(b"&lt;"));
                }
                b'>' => {
                    try!(wr.write_all(b"&gt;"));
                }
                b'&' => {
                    try!(wr.write_all(b"&amp;"));
                }
                b'"' => {
                    try!(wr.write_all(b"&quot;"));
                }
                b'\'' => {
                    try!(wr.write_all(b"&#39;"));
                }
                _ => {
                    try!(wr.write_all(&[b]));
                }
            }
        }

        Ok(())
    }

    fn render_utag<W: Write>(&mut self, wr: &mut W, stack: &mut Vec<&Data>, path: &[String]) -> Result {
        match self.find(path, stack) {
            None => {}
            Some(mut value) => {
                try!(self.write_indent(wr));

                // Currently this doesn't allow Option<Option<Foo>>, which
                // would be un-nameable in the view anyway, so I'm unsure if it's
                // a real problem. Having {{foo}} render only when `foo = Some(Some(val))`
                // seems unintuitive and may be surprising in practice.
                if let OptVal(ref inner) = *value {
                    match *inner {
                        Some(ref inner) => value = inner,
                        None => return Ok(()),
                    }
                }

                match *value {
                    StrVal(ref value) => {
                        try!(self.write_tracking_newlines(wr, value));
                    }

                    // etags and utags use the default delimiter.
                    Fun(ref fcell) => {
                        let f = &mut *fcell.borrow_mut();
                        let tokens = try!(self.render_fun("", "{{", "}}", f));
                        try!(self.render(wr, stack, &tokens));
                    }

                    ref value => {
                        bug!("render_utag: unexpected value {:?}", value);
                    }
                }
            }
        };

        Ok(())
    }

    fn render_inverted_section<W: Write>(&mut self,
                                         wr: &mut W,
                                         stack: &mut Vec<&Data>,
                                         path: &[String],
                                         children: &[Token]) -> Result {
        match self.find(path, stack) {
            None => {}
            Some(&Bool(false)) => {}
            Some(&VecVal(ref xs)) if xs.is_empty() => {}
            Some(&OptVal(ref val)) if val.is_none() => {}
            Some(_) => {
                return Ok(());
            }
        }

        self.render(wr, stack, children)
    }

    fn render_section<W: Write>(&mut self,
                                wr: &mut W,
                                stack: &mut Vec<&Data>,
                                path: &[String],
                                children: &[Token],
                                src: &str,
                                otag: &str,
                                ctag: &str) -> Result {
        match self.find(path, stack) {
            None => {}
            Some(value) => {
                match *value {
                    Bool(true) => {
                        try!(self.render(wr, stack, children));
                    }
                    Bool(false) => {}
                    VecVal(ref vs) => {
                        for v in vs.iter() {
                            stack.push(v);
                            try!(self.render(wr, stack, children));
                            stack.pop();
                        }
                    }
                    Map(_) => {
                        stack.push(value);
                        try!(self.render(wr, stack, children));
                        stack.pop();
                    }
                    Fun(ref fcell) => {
                        let f = &mut *fcell.borrow_mut();
                        let tokens = try!(self.render_fun(src, otag, ctag, f));
                        try!(self.render(wr, stack, &tokens));
                    }
                    OptVal(ref val) => {
                        if let Some(ref val) = *val {
                            stack.push(val);
                            try!(self.render(wr, stack, children));
                            stack.pop();
                        }
                    }
                    StrVal(ref val) => {
                        if val != "" {
                            stack.push(value);
                            try!(self.render(wr, stack, children));
                            stack.pop();
                        }
                    }
                }
            }
        };

        Ok(())
    }

    fn render_partial<W: Write>(&mut self,
                                wr: &mut W,
                                stack: &mut Vec<&Data>,
                                name: &str,
                                indent: &str) -> Result {
        match self.template.partials.get(name) {
            None => {}
            Some(ref tokens) => {
                let mut indent = self.indent.clone() + indent;

                mem::swap(&mut self.indent, &mut indent);
                try!(self.render(wr, stack, &tokens));
                mem::swap(&mut self.indent, &mut indent);
            }
        };

        Ok(())
    }

    fn render_fun(&self,
                  src: &str,
                  otag: &str,
                  ctag: &str,
                  f: &mut Box<FnMut(String) -> String + Send + 'static>)
                  -> Result<Vec<Token>> {
        let src = f(src.to_string());

        let compiler = Compiler::new_with(self.template.ctx.clone(),
                                          src.chars(),
                                          self.template.partials.clone(),
                                          otag.to_string(),
                                          ctag.to_string());

        let (tokens, _) = try!(compiler.compile());
        Ok(tokens)
    }

    fn find<'c>(&self, path: &[String], stack: &mut Vec<&'c Data>) -> Option<&'c Data> {
        // If we have an empty path, we just want the top value in our stack.
        if path.is_empty() {
            match stack.last() {
                None => {
                    return None;
                }
                Some(data) => {
                    return Some(*data);
                }
            }
        }

        // Otherwise, find the stack that has the first part of our path.
        let mut value = None;

        for data in stack.iter().rev() {
            match **data {
                Map(ref m) => {
                    if let Some(v) = m.get(&path[0]) {
                        value = Some(v);
                        break;
                    }
                }
                _ => { /* continue searching the stack */ },
            }
        }

        // Walk the rest of the path to find our final value.
        let mut value = match value {
            Some(value) => value,
            None => {
                return None;
            }
        };

        for part in path[1..].iter() {
            match *value {
                Map(ref m) => {
                    match m.get(part) {
                        Some(v) => {
                            value = v;
                        }
                        None => {
                            return None;
                        }
                    }
                }
                _ => {
                    return None;
                }
            }
        }

        Some(value)
    }
}
