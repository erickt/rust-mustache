extern mod std;
extern mod extra;

use std::char;
use std::hashmap::HashMap;
use std::io::{ignore_io_error, File};
use std::str;
use std::util;
use extra::serialize;

/// Represents template data.
#[deriving(Clone)]
pub enum Data {
    Str(~str),
    Bool(bool),
    Vec(~[Data]),
    Map(HashMap<~str, Data>),
    //Fun(fn(~str) -> ~str),
}

/// Represents the shared metadata needed to compile and render a mustache
/// template.
#[deriving(Clone)]
pub struct Context {
    template_path: Path,
    template_extension: ~str,
}

pub struct Template {
    ctx: Context,
    tokens: ~[Token],
    partials: HashMap<~str, ~[Token]>
}

impl Context {
    /// Configures a mustache context the specified path to the templates.
    fn new(path: Path) -> Context {
        Context {
            template_path: path,
            template_extension: ~"mustache",
        }
    }

    /// Compiles a template from an iterator
    fn compile<IT: Iterator<char>>(&self, rdr: IT) -> Template {
        let mut rdr = rdr;
        let mut ctx = CompileContext {
            rdr: &mut rdr,
            partials: HashMap::new(),
            otag: ~"{{",
            ctag: ~"}}",
            template_path: self.template_path.clone(),
            template_extension: self.template_extension.to_owned(),
        };

        let tokens = ctx.compile();

        Template {
            ctx: self.clone(),
            tokens: tokens,
            partials: ctx.partials,
        }
    }

    fn compile_path(&self, path: Path) -> Option<Template> {
        // FIXME(#6164): This should use the file decoding tools when they are
        // written. For now we'll just read the file and treat it as UTF-8file.
        let mut path = self.template_path.join(path);
        path.set_extension(self.template_extension.clone());

        let s = match File::open(&path) {
            Some(mut rdr) => str::from_utf8_owned(rdr.read_to_end()),
            None => { return None; }
        };

        Some(self.compile(s.chars()))
    }

    /// Renders a template from an iterator.
    fn render<
        IT: Iterator<char>,
        T: serialize::Encodable<Encoder>
    >(&self, rdr: IT, data: &T) -> ~str {
        self.compile(rdr).render(data)
    }
}

/// Compiles a template from an `Iterator<char>`.
pub fn compile_iter<T: Iterator<char>>(iter: T) -> Template {
    Context::new(Path::new(".")).compile(iter)
}

/// Compiles a template from a path.
pub fn compile_path(path: Path) -> Option<Template> {
    Context::new(Path::new(".")).compile_path(path)
}

/// Compiles a template from a string.
pub fn compile_str(template: &str) -> Template {
    Context::new(Path::new(".")).compile(template.chars())
}

/// Renders a template from an `Iterator<char>`.
pub fn render_iter<
    IT: Iterator<char>,
    T: serialize::Encodable<Encoder>
>(rdr: IT, data: &T) -> ~str {
    compile_iter(rdr).render(data)
}

/// Renders a template from a file.
pub fn render_path<
    T: serialize::Encodable<Encoder>
>(path: Path, data: &T) -> Option<~str> {
    compile_path(path).and_then(|template| {
        Some(template.render(data))
    })
}

/// Renders a template from a string.
pub fn render_str<
    T: serialize::Encodable<Encoder>
>(template: &str, data: &T) -> ~str {
    compile_str(template).render(data)
}

pub struct Encoder {
    data: ~[Data],
}

impl Encoder {
    fn new() -> Encoder {
        Encoder { data: ~[] }
    }
}

impl serialize::Encoder for Encoder {
    fn emit_nil(&mut self) { fail!() }

    fn emit_uint(&mut self, v: uint) { self.emit_str(v.to_str()); }
    fn emit_u64(&mut self, v: u64)   { self.emit_str(v.to_str()); }
    fn emit_u32(&mut self, v: u32)   { self.emit_str(v.to_str()); }
    fn emit_u16(&mut self, v: u16)   { self.emit_str(v.to_str()); }
    fn emit_u8(&mut self, v: u8)     { self.emit_str(v.to_str()); }

    fn emit_int(&mut self, v: int) { self.emit_str(v.to_str()); }
    fn emit_i64(&mut self, v: i64) { self.emit_str(v.to_str()); }
    fn emit_i32(&mut self, v: i32) { self.emit_str(v.to_str()); }
    fn emit_i16(&mut self, v: i16) { self.emit_str(v.to_str()); }
    fn emit_i8(&mut self, v: i8)   { self.emit_str(v.to_str()); }

    fn emit_bool(&mut self, v: bool) { self.data.push(Bool(v)); }

    fn emit_f64(&mut self, v: f64) {
        self.emit_str(v.to_str());
    }

    fn emit_f32(&mut self, v: f32) {
        self.emit_str(v.to_str());
    }

    fn emit_char(&mut self, v: char) {
        self.emit_str(str::from_char(v));
    }

    fn emit_str(&mut self, v: &str) {
        // copying emit_owned_str
        self.data.push(Str(v.to_owned()));
    }

    fn emit_enum(&mut self, _name: &str, _f: |&mut Encoder|) {
        fail!()
    }

    fn emit_enum_variant(&mut self, _name: &str, _id: uint, _len: uint, _f: |&mut Encoder|) {
        fail!()
    }

    fn emit_enum_variant_arg(&mut self, _a_idx: uint, _f: |&mut Encoder|) {
        fail!()
    }

    fn emit_enum_struct_variant(&mut self,
                                _v_name: &str,
                                _v_id: uint,
                                _len: uint,
                                _f: |&mut Encoder|) {
        fail!()
    }

    fn emit_enum_struct_variant_field(&mut self,
                                      _f_name: &str,
                                      _f_idx: uint,
                                      _f: |&mut Encoder|) {
        fail!()
    }

    fn emit_struct(&mut self, _name: &str, _len: uint, f: |&mut Encoder|) {
        self.data.push(Map(HashMap::new()));
        f(self);
    }

    fn emit_struct_field(&mut self, name: &str, _idx: uint, f: |&mut Encoder|) {
        let mut m = match self.data.pop() {
            Map(m) => m,
            _ => fail!(),
        };
        f(self);
        m.insert(name.to_owned(), self.data.pop());
        self.data.push(Map(m));
    }

    fn emit_tuple(&mut self, len: uint, f: |&mut Encoder|) {
        self.emit_seq(len, f)
    }

    fn emit_tuple_arg(&mut self, idx: uint, f: |&mut Encoder|) {
        self.emit_seq_elt(idx, f)
    }

    fn emit_tuple_struct(&mut self, _name: &str, len: uint, f: |&mut Encoder|) {
        self.emit_seq(len, f)
    }

    fn emit_tuple_struct_arg(&mut self, idx: uint, f: |&mut Encoder|) {
        self.emit_seq_elt(idx, f)
    }

    // Specialized types:
    fn emit_option(&mut self, _f: |&mut Encoder|) {
        fail!()
    }

    fn emit_option_none(&mut self) {
        fail!()
    }

    fn emit_option_some(&mut self, _f: |&mut Encoder|) {
        fail!()
    }

    fn emit_seq(&mut self, _len: uint, f: |&mut Encoder|) {
        self.data.push(Vec(~[]));
        f(self);
    }

    fn emit_seq_elt(&mut self, _idx: uint, f: |&mut Encoder|) {
        let mut v = match self.data.pop() {
            Vec(v) => v,
            _ => fail!(),
        };
        f(self);
        v.push(self.data.pop());
        self.data.push(Vec(v));
    }

    fn emit_map(&mut self, _len: uint, f: |&mut Encoder|) {
        self.data.push(Map(HashMap::new()));
        f(self);
    }

    fn emit_map_elt_key(&mut self, _idx: uint, f: |&mut Encoder|) {
        f(self);
        match *self.data.last() {
            Str(_) => {}
            _ => fail!("error: key is not a string"),
        }
    }

    fn emit_map_elt_val(&mut self, _idx: uint, f: |&mut Encoder|) {
        let k = match self.data.pop() {
            Str(s) => s,
            _ => fail!(),
        };
        let mut m = match self.data.pop() {
            Map(m) => m,
            _ => fail!(),
        };
        f(self);
        m.insert(k, self.data.pop());
        self.data.push(Map(m));
    }
}

impl Template {
    fn render< T: serialize::Encodable<Encoder> >(&self, data: &T) -> ~str {
        let mut encoder = Encoder::new();
        data.encode(&mut encoder);
        assert_eq!(encoder.data.len(), 1);
        self.render_data(encoder.data.pop())
    }

    fn render_data(&self, data: Data) -> ~str {
        render_helper(&RenderContext {
            ctx: self.ctx.clone(),
            // FIXME: #rust/9382
            // This should be `tokens: self.tokens,` but that's broken
            tokens: self.tokens.as_slice(),
            partials: &self.partials,
            stack: ~[data],
            indent: ~""
        })
    }
}

#[deriving(Clone)]
pub enum Token {
    Text(~str),
    ETag(~[~str], ~str),
    UTag(~[~str], ~str),
    Section(~[~str], bool, ~[Token], ~str, ~str, ~str, ~str, ~str),
    IncompleteSection(~[~str], bool, ~str, bool),
    Partial(~str, ~str, ~str),
}

#[deriving(Clone)]
pub enum TokenClass {
    Normal,
    StandAlone,
    WhiteSpace(~str, uint),
    NewLineWhiteSpace(~str, uint),
}

pub struct Parser<'a, T> {
    rdr: &'a mut T,
    ch: Option<char>,
    lookahead: Option<char>,
    line: uint,
    col: uint,
    content: ~str,
    state: ParserState,
    otag: ~str,
    ctag: ~str,
    otag_chars: ~[char],
    ctag_chars: ~[char],
    tag_position: uint,
    tokens: ~[Token],
    partials: ~[~str],
}

enum ParserState { TEXT, OTAG, TAG, CTAG }

impl<'a, T: Iterator<char>> Parser<'a, T> {
    fn eof(&self) -> bool {
        self.ch.is_none()
    }

    fn bump(&mut self) {
        match self.lookahead.take() {
            None => { self.ch = self.rdr.next(); }
            Some(ch) => { self.ch = Some(ch); }
        }

        match self.ch {
            Some(ch) => {
                if ch == '\n' {
                    self.line += 1;
                    self.col = 1;
                } else {
                    self.col += 1;
                }
            }
            None => { }
        }
    }

    fn peek(&mut self) -> Option<char> {
        match self.lookahead {
            None => {
                self.lookahead = self.rdr.next();
                self.lookahead
            }
            Some(ch) => Some(ch),
        }
    }

    fn ch_is(&self, ch: char) -> bool {
        match self.ch {
            Some(c) => c == ch,
            None => false,
        }
    }

    fn parse(&mut self) {
        let mut curly_brace_tag = false;

        loop {
            let ch = match self.ch {
                Some(ch) => ch,
                None => { break; }
            };

            match self.state {
                TEXT => {
                    if ch == self.otag_chars[0] {
                        if self.otag_chars.len() > 1 {
                            self.tag_position = 1;
                            self.state = OTAG;
                        } else {
                            self.add_text();
                            self.state = TAG;
                        }
                    } else {
                        self.content.push_char(ch);
                    }
                    self.bump();
                }
                OTAG => {
                    if ch == self.otag_chars[self.tag_position] {
                        if self.tag_position == self.otag_chars.len() - 1 {
                            self.add_text();
                            curly_brace_tag = false;
                            self.state = TAG;
                        } else {
                            self.tag_position = self.tag_position + 1;
                        }
                    } else {
                        // We don't have a tag, so add all the tag parts we've seen
                        // so far to the string.
                        self.state = TEXT;
                        self.not_otag();
                        self.content.push_char(ch);
                    }
                    self.bump();
                }
                TAG => {
                    if self.content == ~"" && ch == '{' {
                        curly_brace_tag = true;
                        self.content.push_char(ch);
                        self.bump();
                    } else if curly_brace_tag && ch == '}' {
                        curly_brace_tag = false;
                        self.content.push_char(ch);
                        self.bump();
                    } else if ch == self.ctag_chars[0] {
                        if self.ctag_chars.len() > 1 {
                            self.tag_position = 1;
                            self.state = CTAG;
                            self.bump();
                        } else {
                            self.add_tag();
                            self.state = TEXT;
                        }
                    } else {
                        self.content.push_char(ch);
                        self.bump();
                    }
                }
                CTAG => {
                    if ch == self.ctag_chars[self.tag_position] {
                        if self.tag_position == self.ctag_chars.len() - 1 {
                            self.add_tag();
                            self.state = TEXT;
                        } else {
                            self.state = TAG;
                            self.not_ctag();
                            self.content.push_char(ch);
                            self.bump();
                        }
                    } else {
                        fail!("character {} is not part of CTAG: {}",
                              ch,
                              self.ctag_chars[self.tag_position]);
                    }
                }
            }
        }

        match self.state {
            TEXT => { self.add_text(); }
            OTAG => { self.not_otag(); self.add_text(); }
            TAG => { fail!(~"unclosed tag"); }
            CTAG => { self.not_ctag(); self.add_text(); }
        }

        // Check that we don't have any incomplete sections.
        for token in self.tokens.iter() {
            match *token {
                IncompleteSection(ref path, _, _, _) => {
                    fail!("Unclosed mustache section {}", path.connect("."));
              }
              _ => {}
            }
        };
    }

    fn add_text(&mut self) {
        if self.content != ~"" {
            let mut content = ~"";
            util::swap(&mut content, &mut self.content);

            self.tokens.push(Text(content));
        }
    }

    // This function classifies whether or not a token is standalone, or if it
    // has trailing whitespace. It's looking for this pattern:
    //
    //   ("\n" | "\r\n") whitespace* token ("\n" | "\r\n")
    //
    fn classify_token(&mut self) -> TokenClass {
        // Exit early if the next character is not '\n' or '\r\n'.
        match self.ch {
            None => { }
            Some(ch) => {
                if !(ch == '\n' || (ch == '\r' && self.peek() == Some('\n'))) {
                    return Normal;
                }
            }
        }

        // If the last token ends with a newline (or there is no previous
        // token), then this token is standalone.
        if self.tokens.len() == 0 { return StandAlone; }

        match self.tokens[self.tokens.len() - 1] {
            IncompleteSection(_, _, _, true) => { StandAlone }

            Text(ref s) if !s.is_empty() => {
                // Look for the last newline character that may have whitespace
                // following it.
                match s.rfind(|c:char| c == '\n' || !char::is_whitespace(c)) {
                    // It's all whitespace.
                    None => {
                        if self.tokens.len() == 1 {
                            WhiteSpace(s.to_owned(), 0)
                        } else {
                            Normal
                        }
                    }
                    Some(pos) => {
                        if s.char_at(pos) == '\n' {
                            if pos == s.len() - 1 {
                                StandAlone
                            } else {
                                WhiteSpace(s.to_owned(), pos + 1)
                            }
                        } else { Normal }
                    }
                }
            }
            _ => Normal,
        }
    }

    fn eat_whitespace(&mut self) -> bool {
        // If the next character is a newline, and the last token ends with a
        // newline and whitespace, clear out the whitespace.

        match self.classify_token() {
            Normal => { false }
            StandAlone => {
                if self.ch_is('\r') { self.bump(); }
                self.bump();
                true
            }
            WhiteSpace(s, pos) | NewLineWhiteSpace(s, pos) => {
                if self.ch_is('\r') { self.bump(); }
                self.bump();

                // Trim the whitespace from the last token.
                self.tokens.pop();
                self.tokens.push(Text(s.slice(0, pos).to_str()));

                true
            }
        }
    }

    fn add_tag(&mut self) {
        self.bump();

        let tag = self.otag + self.content + self.ctag;

        // Move the content to avoid a copy.
        let mut content = ~"";
        util::swap(&mut content, &mut self.content);
        let len = content.len();

        match content[0] as char {
            '!' => {
                // ignore comments
                self.eat_whitespace();
            }
            '&' => {
                let name = content.slice(1, len);
                let name = self.check_content(name);
                let name = name.split_terminator('.')
                    .map(|x| x.to_owned())
                    .collect();
                self.tokens.push(UTag(name, tag));
            }
            '{' => {
                if content.ends_with("}") {
                    let name = content.slice(1, len - 1);
                    let name = self.check_content(name);
                    let name = name.split_terminator('.')
                        .map(|x| x.to_owned())
                        .collect();
                    self.tokens.push(UTag(name, tag));
                } else { fail!(~"unbalanced \"{\" in tag"); }
            }
            '#' => {
                let newlined = self.eat_whitespace();

                let name = self.check_content(content.slice(1, len));
                let name = name.split_terminator('.')
                    .map(|x| x.to_owned())
                    .collect();
                self.tokens.push(IncompleteSection(name, false, tag, newlined));
            }
            '^' => {
                let newlined = self.eat_whitespace();

                let name = self.check_content(content.slice(1, len));
                let name = name.split_terminator('.')
                    .map(|x| x.to_owned())
                    .collect();
                self.tokens.push(IncompleteSection(name, true, tag, newlined));
            }
            '/' => {
                self.eat_whitespace();

                let name = self.check_content(content.slice(1, len));
                let name = name.split_terminator('.')
                    .map(|x| x.to_owned())
                    .collect();
                let mut children: ~[Token] = ~[];

                loop {
                    if self.tokens.len() == 0 {
                        fail!(~"closing unopened section");
                    }

                    let last = self.tokens.pop();

                    match last {
                        IncompleteSection(section_name, inverted, osection, _) => {
                            children.reverse();

                            // Collect all the children's sources.
                            let mut srcs = ~[];
                            for child in children.iter() {
                                match *child {
                                    Text(ref s)
                                    | ETag(_, ref s)
                                    | UTag(_, ref s)
                                    | Partial(_, _, ref s) => {
                                        srcs.push(s.clone())
                                    }
                                    Section(_, _, _, _, ref osection, ref src, ref csection, _) => {
                                        srcs.push(osection.clone());
                                        srcs.push(src.clone());
                                        srcs.push(csection.clone());
                                    }
                                    _ => fail!(),
                                }
                            }

                            if section_name == name {
                                // Cache the combination of all the sources in the
                                // section. It's unfortunate, but we need to do this in
                                // case the user uses a function to instantiate the
                                // tag.
                                let mut src = ~"";
                                for s in srcs.iter() { src.push_str(*s); }

                                self.tokens.push(
                                    Section(
                                        name,
                                        inverted,
                                        children,
                                        self.otag.to_owned(),
                                        osection,
                                        src,
                                        tag,
                                        self.ctag.to_owned()));
                                break;
                            } else {
                                fail!(~"Unclosed section");
                            }
                        }
                        _ => { children.push(last); }
                    }
                }
            }
            '>' => { self.add_partial(content, tag); }
            '=' => {
                self.eat_whitespace();

                if (len > 2u && content.ends_with("=")) {
                    let s = self.check_content(content.slice(1, len - 1));

                    let pos = s.find(char::is_whitespace);
                    let pos = match pos {
                      None => { fail!("invalid change delimiter tag content"); }
                      Some(pos) => { pos }
                    };

                    self.otag = s.slice(0, pos).to_str();
                    self.otag_chars = self.otag.chars().collect();

                    let s2 = s.slice_from(pos);
                    let pos = s2.find(|c| !char::is_whitespace(c));
                    let pos = match pos {
                      None => { fail!("invalid change delimiter tag content"); }
                      Some(pos) => { pos }
                    };

                    self.ctag = s2.slice_from(pos).to_str();
                    self.ctag_chars = self.ctag.chars().collect();
                } else {
                    fail!("invalid change delimiter tag content");
                }
            }
            _ => {
                // If the name is "." then we want the top element, which we represent with
                // an empty name.
                let name = match self.check_content(content) {
                    ~"." => ~[],
                    name => {
                        name.split_terminator('.')
                            .map(|x| x.to_owned())
                            .collect()
                    }
                };

                self.tokens.push(ETag(name, tag));
            }
        }
    }

    fn add_partial(&mut self, content: &str, tag: ~str) {
        let indent = match self.classify_token() {
            Normal => ~"",
            StandAlone => {
                if self.ch_is('\r') { self.bump(); }
                self.bump();
                ~""
            }
            WhiteSpace(s, pos) | NewLineWhiteSpace(s, pos) => {
                if self.ch_is('\r') { self.bump(); }
                self.bump();

                let ws = s.slice(pos, s.len());

                // Trim the whitespace from the last token.
                self.tokens.pop();
                self.tokens.push(Text(s.slice(0, pos).to_str()));

                ws.to_owned()
            }
        };

        // We can't inline the tokens directly as we may have a recursive
        // partial. So instead, we'll cache the partials we used and look them
        // up later.
        let name = content.slice(1, content.len());
        let name = self.check_content(name);

        self.tokens.push(Partial(name.to_owned(), indent, tag));
        self.partials.push(name);
    }

    fn not_otag(&mut self) {
        let mut i = 0;
        while i < self.tag_position {
            self.content.push_char(self.otag_chars[i]);
            i += 1;
        }
    }

    fn not_ctag(&mut self) {
        let mut i = 0;
        while i < self.tag_position {
            self.content.push_char(self.ctag_chars[i]);
            i += 1;
        }
    }

    fn check_content(&self, content: &str) -> ~str {
        let trimmed = content.trim();
        if trimmed.len() == 0 {
            fail!(~"empty tag");
        }
        trimmed.to_owned()
    }
}

struct CompileContext<'a, T> {
    rdr: &'a mut T,
    partials: HashMap<~str, ~[Token]>,
    otag: ~str,
    ctag: ~str,
    template_path: Path,
    template_extension: ~str,
}

impl<'a, T: Iterator<char>> CompileContext<'a, T> {
    fn compile(&mut self) -> ~[Token] {
        let mut parser = Parser {
            rdr: self.rdr,
            ch: None,
            lookahead: None,
            line: 1,
            col: 1,
            content: ~"",
            state: TEXT,
            otag: self.otag.to_owned(),
            ctag: self.ctag.to_owned(),
            otag_chars: self.otag.chars().collect::<~[char]>(),
            ctag_chars: self.ctag.chars().collect::<~[char]>(),
            tag_position: 0,
            tokens: ~[],
            partials: ~[],
        };

        parser.bump();
        parser.parse();

        // Compile the partials if we haven't done so already.
        for name in parser.partials.iter() {
            let path = self.template_path.join(*name + "." + self.template_extension);

            if !self.partials.contains_key(name) {
                // Insert a placeholder so we don't recurse off to infinity.
                self.partials.insert(name.to_owned(), ~[]);

                let _ignore = ignore_io_error();
                match File::open(&path) {
                    Some(mut rdr) => {
                        // XXX: HACK
                        let s = str::from_utf8_owned(rdr.read_to_end());
                        let mut iter = s.chars();

                        let mut inner_ctx = CompileContext {
                            rdr: &mut iter,
                            partials: self.partials.clone(),
                            otag: ~"{{",
                            ctag: ~"}}",
                            template_path: self.template_path.clone(),
                            template_extension: self.template_extension.to_owned(),
                        };
                        let tokens = inner_ctx.compile();

                        self.partials.insert(name.to_owned(), tokens);
                    }
                    None => { }
                }
            }
        }

        // Destructure the parser so we get get at the tokens without a copy.
        let Parser { tokens: tokens, .. } = parser;

        tokens
    }
}

#[deriving(Clone)]
struct RenderContext<'a> {
    ctx: Context,
    tokens: &'a [Token],
    partials: &'a HashMap<~str, ~[Token]>,
    stack: ~[Data],
    indent: ~str,
}

fn render_helper(ctx: &RenderContext) -> ~str {
    fn find(stack: &[Data], path: &[~str]) -> Option<Data> {
        // If we have an empty path, we just want the top value in our stack.
        if path.is_empty() {
            return match stack.last_opt() {
                None => None,
                Some(value) => Some(value.clone()),
            };
        }

        // Otherwise, find the stack that has the first part of our path.
        let mut value: Option<Data> = None;

        let mut i = stack.len();
        while i > 0 {
            match stack[i - 1] {
                Map(ref ctx) => {
                    match ctx.find_equiv(&path[0]) {
                        Some(v) => { value = Some(v.clone()); break; }
                        None => {}
                    }
                    i -= 1;
                }
                _ => {
                    fail!("{:?} {:?}", stack, path)
                }
            }
        }

        // Walk the rest of the path to find our final value.
        let mut value = value;

        let mut i = 1;
        let len = path.len();

        while i < len {
            value = match value {
                Some(Map(v)) => {
                    match v.find_equiv(&path[i]) {
                        Some(value) => Some(value.clone()),
                        None => None,
                    }
                }
                _ => break,
            };
            i = i + 1;
        }

        value
    }

    let mut output = ~"";

    for token in ctx.tokens.iter() {
        match *token {
            Text(ref value) => {
                // Indent the lines.
                if ctx.indent.equiv(& &"") {
                    output = output + *value;
                } else {
                    let mut pos = 0;
                    let len = value.len();

                    while pos < len {
                        let v = value.slice_from(pos);
                        let line = match v.find('\n') {
                            None => {
                                let line = v;
                                pos = len;
                                line
                            }
                            Some(i) => {
                                let line = v.slice_to(i + 1);
                                pos += i + 1;
                                line
                            }
                        };

                        if line.char_at(0) != '\n' {
                            output.push_str(ctx.indent);
                        }

                        output.push_str(line);
                    }
                }
            },
            ETag(ref path, _) => {
                match find(ctx.stack, *path) {
                    None => { }
                    Some(value) => {
                        output = output + ctx.indent + render_etag(value, ctx);
                    }
                }
            }
            UTag(ref path, _) => {
                match find(ctx.stack, *path) {
                    None => { }
                    Some(value) => {
                        output = output + ctx.indent + render_utag(value, ctx);
                    }
                }
            }
            Section(ref path, true, ref children, _, _, _, _, _) => {
                let ctx = RenderContext {
                    // FIXME: #rust/9382
                    // This should be `tokens: *children,` but that's broken
                    tokens: children.as_slice(),
                    .. ctx.clone()
                };

                output = output + match find(ctx.stack, *path) {
                    None => { render_helper(&ctx) }
                    Some(value) => { render_inverted_section(value, &ctx) }
                };
            }
            Section(ref path, false, ref children, ref otag, _, ref src, _, ref ctag) => {
                match find(ctx.stack, *path) {
                    None => { }
                    Some(value) => {
                        output = output + render_section(
                            value,
                            *src,
                            *otag,
                            *ctag,
                            &RenderContext {
                                // FIXME: #rust/9382
                                // This should be `tokens: *children,` but that's broken
                                tokens: children.as_slice(),
                                .. ctx.clone()
                            }
                        );
                    }
                }
            }
            Partial(ref name, ref ind, _) => {
                match ctx.partials.find(name) {
                    None => { }
                    Some(tokens) => {
                        output = output + render_helper(&RenderContext {
                            // FIXME: #rust/9382
                            // This should be `tokens: *tokens,` but that's broken
                            tokens: tokens.as_slice(),
                            indent: ctx.indent + *ind,
                            .. ctx.clone()
                        });
                    }
                }
            }
            _ => { fail!() }
        };
    };

    output
}

fn render_etag(value: Data, ctx: &RenderContext) -> ~str {
    let mut escaped = ~"";
    let utag = render_utag(value, ctx);
    for c in utag.chars() {
        match c {
            '<' => { escaped.push_str("&lt;"); }
            '>' => { escaped.push_str("&gt;"); }
            '&' => { escaped.push_str("&amp;"); }
            '"' => { escaped.push_str("&quot;"); }
            '\'' => { escaped.push_str("&#39;"); }
            _ => { escaped.push_char(c); }
        }
    }
    escaped
}

fn render_utag(value: Data, _ctx: &RenderContext) -> ~str {
    match value {
        Str(ref s) => s.clone(),

        // etags and utags use the default delimiter.
        //Fun(f) => render_fun(ctx, ~"", ~"{{", ~"}}", f),

        _ => fail!(),
    }
}

fn render_inverted_section(value: Data, ctx: &RenderContext) -> ~str {
    match value {
        Bool(false) => render_helper(ctx),
        Vec(ref xs) if xs.len() == 0 => render_helper(ctx),
        _ => ~"",
    }
}

fn render_section(value: Data,
                  _src: &str,
                  _otag: &str,
                  _ctag: &str,
                  ctx: &RenderContext) -> ~str {
    match value {
        Bool(true) => render_helper(ctx),
        Bool(false) => ~"",
        Vec(vs) => {
            vs.map(|v| {
                let mut stack = ctx.stack.to_owned();
                stack.push(v.clone());

                render_helper(&RenderContext { stack: stack, .. (*ctx).clone() })
            }).concat()
        }
        Map(_) => {
            let mut stack = ctx.stack.to_owned();
            stack.push(value);

            render_helper(&RenderContext { stack: stack, .. (*ctx).clone() })
        }
        //Fun(f) => render_fun(ctx, src, otag, ctag, f),
        _ => fail!(),
    }
}

fn render_fun(ctx: &RenderContext,
              src: &str,
              otag: &str,
              ctag: &str,
              f: |&str| -> ~str) -> ~str {
    let src = f(src);
    let mut iter = src.chars();

    let mut inner_ctx = CompileContext {
        rdr: &mut iter,
        partials: ctx.partials.clone(),
        otag: otag.to_owned(),
        ctag: ctag.to_owned(),
        template_path: ctx.ctx.template_path.clone(),
        template_extension: ctx.ctx.template_extension.to_owned(),
    };
    let tokens = inner_ctx.compile();

    render_helper(&RenderContext {
        // FIXME: #rust/9382
        // This should be `tokens: tokens,` but that's broken
        tokens: tokens.as_slice(),
        .. ctx.clone()
    })
}

#[cfg(test)]
mod tests {
    use std::str;
    use std::hashmap::HashMap;
    use std::io::File;
    use extra::json;
    use extra::serialize::Encodable;
    use extra::serialize;
    use extra::tempfile;
    use super::{compile_str, render_str};
    use super::{Context, Encoder};
    use super::{Data, Str, Vec, Map};
    use super::{Token, Text, ETag, UTag, Section, Partial};

    fn token_to_str(token: &Token) -> ~str {
        match *token {
            // recursive enums crash %?
            Section(ref name,
                    inverted,
                    ref children,
                    ref otag,
                    ref osection,
                    ref src,
                    ref tag,
                    ref ctag) => {
                let children = children.map(|x| token_to_str(x));
                format!("Section({:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?})",
                        name,
                        inverted,
                        children,
                        otag,
                        osection,
                        src,
                        tag,
                        ctag)
            }
            _ => {
                format!("{:?}", token)
            }
        }
    }

    fn check_tokens(actual: &[Token], expected: &[Token]) -> bool {
        // TODO: equality is currently broken for enums
        let actual = do actual.map |x| {token_to_str(x)};
        let expected = do expected.map |x| {token_to_str(x)};
        if actual !=  expected {
            error!("Found {:?}, but expected {:?}", actual, expected);
            return false;
        }
        return true;
    }

    #[test]
    fn test_compile_texts() {
        assert!(check_tokens(compile_str("hello world").tokens, [Text(~"hello world")]));
        assert!(check_tokens(compile_str("hello {world").tokens, [Text(~"hello {world")]));
        assert!(check_tokens(compile_str("hello world}").tokens, [Text(~"hello world}")]));
        assert!(check_tokens(compile_str("hello world}}").tokens, [Text(~"hello world}}")]));
    }

    #[test]
    fn test_compile_etags() {
        assert!(check_tokens(compile_str("{{ name }}").tokens, [
            ETag(~[~"name"], ~"{{ name }}")
        ]));

        assert!(check_tokens(compile_str("before {{name}} after").tokens, [
            Text(~"before "),
            ETag(~[~"name"], ~"{{name}}"),
            Text(~" after")
        ]));

        assert!(check_tokens(compile_str("before {{name}}").tokens, [
            Text(~"before "),
            ETag(~[~"name"], ~"{{name}}")
        ]));

        assert!(check_tokens(compile_str("{{name}} after").tokens, [
            ETag(~[~"name"], ~"{{name}}"),
            Text(~" after")
        ]));
    }

    #[test]
    fn test_compile_utags() {
        assert!(check_tokens(compile_str("{{{name}}}").tokens, [
            UTag(~[~"name"], ~"{{{name}}}")
        ]));

        assert!(check_tokens(compile_str("before {{{name}}} after").tokens, [
            Text(~"before "),
            UTag(~[~"name"], ~"{{{name}}}"),
            Text(~" after")
        ]));

        assert!(check_tokens(compile_str("before {{{name}}}").tokens, [
            Text(~"before "),
            UTag(~[~"name"], ~"{{{name}}}")
        ]));

        assert!(check_tokens(compile_str("{{{name}}} after").tokens, [
            UTag(~[~"name"], ~"{{{name}}}"),
            Text(~" after")
        ]));
    }

    #[test]
    fn test_compile_sections() {
        assert!(check_tokens(compile_str("{{# name}}{{/name}}").tokens, [
            Section(
                ~[~"name"],
                false,
                ~[],
                ~"{{",
                ~"{{# name}}",
                ~"",
                ~"{{/name}}",
                ~"}}"
            )
        ]));

        assert!(check_tokens(compile_str("before {{^name}}{{/name}} after").tokens, [
            Text(~"before "),
            Section(
                ~[~"name"],
                true,
                ~[],
                ~"{{",
                ~"{{^name}}",
                ~"",
                ~"{{/name}}",
                ~"}}"
            ),
            Text(~" after")
        ]));

        assert!(check_tokens(compile_str("before {{#name}}{{/name}}").tokens, [
            Text(~"before "),
            Section(
                ~[~"name"],
                false,
                ~[],
                ~"{{",
                ~"{{#name}}",
                ~"",
                ~"{{/name}}",
                ~"}}"
            )
        ]));

        assert!(check_tokens(compile_str("{{#name}}{{/name}} after").tokens, [
            Section(
                ~[~"name"],
                false,
                ~[],
                ~"{{",
                ~"{{#name}}",
                ~"",
                ~"{{/name}}",
                ~"}}"
            ),
            Text(~" after")
        ]));

        assert!(check_tokens(compile_str(
                "before {{#a}} 1 {{^b}} 2 {{/b}} {{/a}} after").tokens, [
            Text(~"before "),
            Section(
                ~[~"a"],
                false,
                ~[
                    Text(~" 1 "),
                    Section(
                        ~[~"b"],
                        true,
                        ~[Text(~" 2 ")],
                        ~"{{",
                        ~"{{^b}}",
                        ~" 2 ",
                        ~"{{/b}}",
                        ~"}}"
                    ),
                    Text(~" ")
                ],
                ~"{{",
                ~"{{#a}}",
                ~" 1 {{^b}} 2 {{/b}} ",
                ~"{{/a}}",
                ~"}}"
            ),
            Text(~" after")
        ]));
    }

    #[test]
    fn test_compile_partials() {
        assert!(check_tokens(compile_str("{{> test}}").tokens, [
            Partial(~"test", ~"", ~"{{> test}}")
        ]));

        assert!(check_tokens(compile_str("before {{>test}} after").tokens, [
            Text(~"before "),
            Partial(~"test", ~"", ~"{{>test}}"),
            Text(~" after")
        ]));

        assert!(check_tokens(compile_str("before {{> test}}").tokens, [
            Text(~"before "),
            Partial(~"test", ~"", ~"{{> test}}")
        ]));

        assert!(check_tokens(compile_str("{{>test}} after").tokens, [
            Partial(~"test", ~"", ~"{{>test}}"),
            Text(~" after")
        ]));
    }

    #[test]
    fn test_compile_delimiters() {
        assert!(check_tokens(compile_str("before {{=<% %>=}}<%name%> after").tokens, [
            Text(~"before "),
            ETag(~[~"name"], ~"<%name%>"),
            Text(~" after")
        ]));
    }

    #[deriving(Encodable)]
    struct Name { name: ~str }

    #[test]
    fn test_render_texts() {
        let ctx = &Name { name: ~"world" };

        assert_eq!(render_str("hello world", ctx), ~"hello world");
        assert_eq!(render_str("hello {world", ctx), ~"hello {world");
        assert_eq!(render_str("hello world}", ctx), ~"hello world}");
        assert_eq!(render_str("hello {world}", ctx), ~"hello {world}");
        assert_eq!(render_str("hello world}}", ctx), ~"hello world}}");
    }

    #[test]
    fn test_render_etags() {
        let ctx = &Name { name: ~"world" };

        assert!(render_str("hello {{name}}", ctx) == ~"hello world");
    }

    #[test]
    fn test_render_utags() {
        let ctx = &Name { name: ~"world" };

        assert!(render_str("hello {{{name}}}", ctx) == ~"hello world");
    }

    struct StrHash<V>(HashMap<~str, V>);

    impl<E: serialize::Encoder, V> serialize::Encodable<E> for StrHash<V> {
        fn encode(&self, e: &mut E) {
            (*self).encode(e)
        }
    }

    #[test]
    fn test_render_sections() {
        let mut ctx0 = HashMap::new();
        let template = compile_str("0{{#a}}1 {{n}} 3{{/a}}5");

        assert!(template.render_data(Map(ctx0.clone())) == ~"05");

        ctx0.insert(~"a", Vec(~[]));
        assert!(template.render_data(Map(ctx0.clone())) == ~"05");

        let ctx1: HashMap<~str, Data> = HashMap::new();
        ctx0.insert(~"a", Vec(~[Map(ctx1.clone())]));

        assert!(template.render_data(Map(ctx0.clone())) == ~"01  35");

        let mut ctx1 = HashMap::new();
        ctx1.insert(~"n", Str(~"a"));
        ctx0.insert(~"a", Vec(~[Map(ctx1.clone())]));
        assert!(template.render_data(Map(ctx0.clone())) == ~"01 a 35");

        //ctx0.insert(~"a", Fun(|_text| {~"foo"}));
        //assert!(template.render_data(Map(ctx0)) == ~"0foo5");
    }

    #[test]
    fn test_render_inverted_sections() {
        let template = compile_str("0{{^a}}1 3{{/a}}5");

        let mut ctx0 = HashMap::new();
        assert!(template.render_data(Map(ctx0.clone())) == ~"01 35");

        ctx0.insert(~"a", Vec(~[]));
        assert!(template.render_data(Map(ctx0.clone())) == ~"01 35");

        let mut ctx1 = HashMap::new();
        ctx0.insert(~"a", Vec(~[Map(ctx1.clone())]));
        assert!(template.render_data(Map(ctx0.clone())) == ~"05");

        ctx1.insert(~"n", Str(~"a"));
        assert!(template.render_data(Map(ctx0.clone())) == ~"05");
    }

    #[test]
    fn test_render_partial() {
        let template = Context::new(Path::new("src/test-data"))
            .compile_path(Path::new("base"))
            .unwrap();

        let mut ctx0 = HashMap::new();
        assert_eq!(template.render_data(Map(ctx0.clone())), ~"<h2>Names</h2>\n");

        ctx0.insert(~"names", Vec(~[]));
        assert_eq!(template.render_data(Map(ctx0.clone())), ~"<h2>Names</h2>\n");

        let mut ctx1 = HashMap::new();
        ctx0.insert(~"names", Vec(~[Map(ctx1.clone())]));
        assert_eq!(
            template.render_data(Map(ctx0.clone())),
            ~"<h2>Names</h2>\n  <strong></strong>\n\n");

        ctx1.insert(~"name", Str(~"a"));
        ctx0.insert(~"names", Vec(~[Map(ctx1.clone())]));
        assert_eq!(
            template.render_data(Map(ctx0.clone())),
            ~"<h2>Names</h2>\n  <strong>a</strong>\n\n");

        let mut ctx2 = HashMap::new();
        ctx2.insert(~"name", Str(~"<b>"));
        ctx0.insert(~"names", Vec(~[Map(ctx1), Map(ctx2)]));
        assert_eq!(
            template.render_data(Map(ctx0)),
            ~"<h2>Names</h2>\n  <strong>a</strong>\n\n  <strong>&lt;b&gt;</strong>\n\n");
    }

    fn parse_spec_tests(src: &str) -> ~[json::Json] {
        let path = Path::new(src);

        let mut rdr = match File::open(&path) {
            Some(rdr) => rdr,
            None => fail!(),
        };

        let s = str::from_utf8_owned(rdr.read_to_end());

        match json::from_str(s) {
            Err(e) => fail!(e.to_str()),
            Ok(json) => {
                match json {
                    json::Object(d) => {
                        let mut d = d;
                        match d.pop(&~"tests") {
                            Some(json::List(tests)) => tests,
                            _ => fail!("{}: tests key not a list", src),
                        }
                    }
                    _ => fail!("{}: JSON value not a map", src),
                }
            }
        }
    }

//    fn convert_json_map(map: json::Object) -> HashMap<~str, Data> {
//        let mut d = HashMap::new();
//        for (key, value) in map.move_iter() {
//            d.insert(key.to_owned(), convert_json(value));
//        }
//        d
//    }
//
//    fn convert_json(value: json::Json) -> Data {
//        match value {
//          json::Number(n) => {
//            // We have to cheat and use {:?} because %f doesn't convert 3.3 to
//            // 3.3.
//            Str(fmt!("{:?}", n))
//          }
//          json::String(s) => { Str(s.to_owned()) }
//          json::Boolean(b) => { Bool(b) }
//          json::List(v) => { Vec(v.map(convert_json)) }
//          json::Object(d) => { Map(convert_json_map(d)) }
//          _ => { fail!("{:?}", value) }
//        }
//    }

    fn write_partials(tmpdir: &Path, value: &json::Json) {
        match value {
            &json::Object(ref d) => {
                for (key, value) in d.iter() {
                    match value {
                        &json::String(ref s) => {
                            let mut path = tmpdir.clone();
                            path.push(*key + ".mustache");

                            match File::create(&path) {
                                Some(mut wr) => wr.write(s.as_bytes()),
                                None => fail!(),
                            }
                        }
                        _ => fail!(),
                    }
                }
            },
            _ => fail!(),
        }
    }

    fn run_test(test: ~json::Object, data: Data) {
        let template = match test.find(&~"template") {
            Some(&json::String(ref s)) => s.clone(),
            _ => fail!(),
        };

        let expected = match test.find(&~"expected") {
            Some(&json::String(ref s)) => s.clone(),
            _ => fail!(),
        };

        // Make a temporary dir where we'll store our partials. This is to
        // avoid a race on filenames.
        let tmpdir = match tempfile::TempDir::new("") {
            Some(tmpdir) => tmpdir,
            None => fail!(),
        };

        match test.find(&~"partials") {
            Some(value) => write_partials(tmpdir.path(), value),
            None => {},
        }

        let ctx = Context::new(tmpdir.path().clone());
        let template = ctx.compile(template.iter());
        let result = template.render_data(data);

        if result != expected {
            fn to_list(x: &json::Json) -> json::Json {
                match x {
                    &json::Object(ref d) => {
                        let mut xs = ~[];
                        for (k, v) in d.iter() {
                            let k = json::String(k.clone());
                            let v = to_list(v);
                            xs.push(json::List(~[k, v]));
                        }
                        json::List(xs)
                    },
                    &json::List(ref xs) => {
                        json::List(xs.map(|x| to_list(x)))
                    },
                    _ => { x.clone() }
                }
            }

            println!("desc:     {}", test.find(&~"desc").unwrap().to_str());
            println!("context:  {}", test.find(&~"data").unwrap().to_str());
            println!("=>");
            println!("template: {:?}", template);
            println!("expected: {:?}", expected);
            println!("actual:   {:?}", result);
            println!("");
        }
        assert_eq!(result, expected);
    }

    fn run_tests(spec: &str) {
        for json in parse_spec_tests(spec).move_iter() {
            let test = match json {
                json::Object(m) => m,
                _ => fail!(),
            };

            let data = match test.find(&~"data") {
                Some(data) => data.clone(),
                None => fail!(),
            };

            let mut encoder = Encoder::new();
            data.encode(&mut encoder);
            assert_eq!(encoder.data.len(), 1);

            run_test(test, encoder.data.pop());
        }
    }

    #[test]
    fn test_spec_comments() {
        run_tests("spec/specs/comments.json");
    }

    #[test]
    fn test_spec_delimiters() {
        run_tests("spec/specs/delimiters.json");
    }

    #[test]
    fn test_spec_interpolation() {
        run_tests("spec/specs/interpolation.json");
    }

    #[test]
    fn test_spec_inverted() {
        run_tests("spec/specs/inverted.json");
    }

    #[test]
    fn test_spec_partials() {
        run_tests("spec/specs/partials.json");
    }

    #[test]
    fn test_spec_sections() {
        run_tests("spec/specs/sections.json");
    }

//    #[test]
//    fn test_spec_lambdas() {
//        for json in parse_spec_tests(~"spec/specs/~lambdas.json").iter() {
//            let test = match json {
//                &json::Object(m) => m,
//                _ => fail!(),
//            };
//
//            // Replace the lambda with rust code.
//            let data = match test.find(&~"data") {
//                Some(data) => (*data).clone(),
//                None => fail!(),
//            };
//
//            let encoder = Encoder::new();
//            data.encode(&encoder);
//            let ctx = match encoder.data {
//                [Map(ctx)] => ctx,
//                _ => fail!(),
//            };
//
//            let f = match *test.find(&~"name").unwrap() {
//                json::String(~"Interpolation") => {
//                    |_text| {~"world" }
//                }
//                json::String(~"Interpolation - Expansion") => {
//                    |_text| {~"{{planet}}" }
//                }
//                json::String(~"Interpolation - Alternate Delimiters") => {
//                    |_text| {~"|planet| => {{planet}}" }
//                }
//                json::String(~"Interpolation - Multiple Calls") => {
//                    let calls = 0i;
//                    |_text| {*calls = *calls + 1; calls.to_str() }
//                }
//                json::String(~"Escaping") => {
//                    |_text| {~">" }
//                }
//                json::String(~"Section") => {
//                    |text: ~str| {if *text == ~"{{x}}" { ~"yes" } else { ~"no" } }
//                }
//                json::String(~"Section - Expansion") => {
//                    |text: ~str| {*text + "{{planet}}" + *text }
//                }
//                json::String(~"Section - Alternate Delimiters") => {
//                    |text: ~str| {*text + "{{planet}} => |planet|" + *text }
//                }
//                json::String(~"Section - Multiple Calls") => {
//                    |text: ~str| {~"__" + *text + ~"__" }
//                }
//                json::String(~"Inverted Section") => {
//                    |_text| {~"" }
//                }
//                value => { fail!("{:?}", value) }
//            };
//            //ctx.insert(~"lambda", Fun(f));
//
//            run_test(test, Map(ctx));
//        }
//    }
}
