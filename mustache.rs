use std;

import result::{ok, err};
import io::{reader_util, writer_util};
import std::map::{hashmap, str_hash};

export template;
export compile_reader;
export compile_file;
export compile_str;
export render_reader;
export render_file;
export render_str;
export data;
export str;
export bool;
export vec;
export map;
export fun;

/*
Enum: data

Represents template data.
*/
enum data {
    // Variant: str
    str(str),
    // Variant: bool
    bool(bool),
    // Variant: vec
    vec([data]),
    // Variant: map
    map(context),
    // Variant: fun
    fun(fn@(str) -> str),
}

/*
Function: compile_reader

Compiles a template from an io::reader.
*/
fn compile_reader(rdr: io::reader) -> template {
    let partials = str_hash();
    let tokens = compile_helper(rdr, partials, "{{", "}}");

    { tokens: tokens, partials: partials }
}

/*
Function: compile_file

Compiles a template from a file.
*/
fn compile_file(file: str) -> template {
    alt io::file_reader(file) {
      ok(rdr) { compile_reader(rdr) }
      err(e) { fail e; }
    }
}

/*
Function: compile_str

Compiles a template from a string.
*/
fn compile_str(src: str) -> template {
    io::with_str_reader(src, compile_reader)
}

/*
Function: render_reader

Renders a template from an io::reader.
*/
fn render_reader(rdr: io::reader, context: context) -> str {
    render(compile_reader(rdr), context)
}

/*
Function: render_file

Renders a template from a file.
*/
fn render_file(file: str, context: context) -> str {
    alt io::file_reader(file) {
      ok(rdr) { render_reader(rdr, context) }
      err(e) { fail e; }
    }
}

/*
Function: render_str

Renders a template from a string.
*/
fn render_str(template: str, context: context) -> str {
    render(compile_str(template), context)
}

enum token {
    text(str),
    etag([str], str),
    utag([str], str),
    section([str], bool, [token], str, str, str, str, str),
    incomplete_section([str], bool, str, bool),
    partial(str, str, str),
}

enum token_class {
    normal,
    standalone,
    whitespace(str, uint),
    newline_whitespace(str, uint),
}

type partial_map = hashmap<str, [token]>;

type template = {
    tokens: [token],
    partials: partial_map,
};

type parser = {
    rdr: io::reader,
    mut ch: char,
    mut lookahead: option<char>,
    mut line: uint,
    mut col: uint,
    mut content: str,
    mut state: parser::state,
    mut otag: str,
    mut ctag: str,
    mut otag_chars: [char],
    mut ctag_chars: [char],
    mut tag_position: uint,
    mut tokens: [token],
    mut partials: [str],
};

mod parser {
    enum state { TEXT, OTAG, TAG, CTAG }
}

type context = hashmap<str, data>;

type render_context = {
    tokens: [token],
    partials: partial_map,
    stack: [data],
    indent: str,
};

impl parser for parser {
    fn eof() -> bool { self.ch == -1 as char }

    fn bump() {
        alt self.lookahead {
          none { self.ch = self.rdr.read_char(); }
          some(ch) { self.ch = ch; self.lookahead = none; }
        }

        if self.ch == '\n' {
            self.line += 1u;
            self.col = 1u;
        } else {
            self.col += 1u;
        }
    }

    fn peek() -> char {
        alt self.lookahead {
          none {
            let ch = self.rdr.read_char();
            self.lookahead = some(ch);
            ch
          }
          some(ch) { ch }
        }
    }

    fn parse() -> ([token], [str]) {
        let mut curly_brace_tag = false;

        while !self.eof() {
            alt self.state {
              parser::TEXT {
                if self.ch == self.otag_chars[0] {
                    if vec::len(self.otag_chars) > 1u {
                        self.tag_position = 1u;
                        self.state = parser::OTAG;
                    } else {
                        self.add_text();
                        self.state = parser::TAG;
                    }
                } else {
                    unsafe { str::push_char(self.content, self.ch) };
                }
                self.bump();
              }
              parser::OTAG {
                if self.ch == self.otag_chars[self.tag_position] {
                    if self.tag_position == vec::len(self.otag_chars) - 1u {
                        self.add_text();
                        curly_brace_tag = false;
                        self.state = parser::TAG;
                    } else {
                        self.tag_position += 1u;
                    }
                } else {
                    // We don't have a tag, so add all the tag parts we've seen
                    // so far to the string.
                    self.state = parser::TEXT;
                    self.not_otag();
                    unsafe { str::push_char(self.content, self.ch) };
                }
                self.bump();
              }
              parser::TAG {
                if self.content == "" && self.ch == '{' {
                    curly_brace_tag = true;
                    unsafe { str::push_char(self.content, self.ch) };
                    self.bump();
                } else if curly_brace_tag && self.ch == '}' {
                    curly_brace_tag = false;
                    unsafe { str::push_char(self.content, self.ch) };
                    self.bump();
                } else if self.ch == self.ctag_chars[0u] {
                    if vec::len(self.ctag_chars) > 1u {
                        self.tag_position = 1u;
                        self.state = parser::CTAG;
                        self.bump();
                    } else {
                        self.add_tag();
                        self.state = parser::TEXT;
                    }
                } else {
                    unsafe { str::push_char(self.content, self.ch) };
                    self.bump();
                }
              }
              parser::CTAG {
                if self.ch == self.ctag_chars[self.tag_position] {
                    if self.tag_position == vec::len(self.ctag_chars) - 1u {
                        self.add_tag();
                        self.state = parser::TEXT;
                    } else {
                        self.state = parser::TAG;
                        self.not_ctag();
                        unsafe { str::push_char(self.content, self.ch) };
                        self.bump();
                    }
                }
              }
            }
        }

        alt self.state {
          parser::TEXT { self.add_text() }
          parser::OTAG { self.not_otag(); self.add_text() }
          parser::TAG { fail "unclosed tag" }
          parser::CTAG { self.not_ctag(); self.add_text() }
        }

        // Check that we don't have any incomplete sections.
        vec::iter(copy self.tokens) { |token|
            alt token {
              incomplete_section(name, _, _, _) {
                  fail #fmt("Unclosed mustache section %s",
                    str::connect(name, "."));
              }
              _ {}
            }
        };

        (self.tokens, self.partials)
    }

    fn add_text() {
        if self.content != "" {
            vec::push(self.tokens, text(self.content));
            self.content = "";
        }
    }

    // This function classifies whether or not a token is standalone, or if it
    // has trailing whitespace. It's looking for this pattern:
    //
    //   ("\n" | "\r\n") whitespace* token ("\n" | "\r\n")
    //
    fn classify_token() -> token_class {
        // Exit early if the next character is not '\n' or '\r\n'.
        if self.eof() ||
           self.ch == '\n' ||
           (self.ch == '\r' || self.peek() == '\n') {

            // If the last token ends with a newline (or there is no previous
            // token), then this token is standalone.
            alt vec::last_opt(self.tokens) {
              none | some(incomplete_section(_, _, _, true)) { standalone }

              some(text(s)) if s != "" {
                // Look for the last newline character that may have whitespace
                // following it.
                alt str::rfind(s, {|c| c == '\n' || !char::is_whitespace(c)}) {
                  // It's all whitespace.
                  none {
                    if self.tokens.len() == 1u {
                        whitespace(s, 0u)
                    } else {
                        normal
                    }
                  }
                  some(pos) {
                    if str::char_at(s, pos) == '\n' {
                        if pos == str::len(s) - 1u {
                            standalone
                        } else {
                            whitespace(s, pos + 1u)
                        }
                    } else { normal }
                  }
                }
              }
              _ { normal }
            }
        } else { normal }
    }

    fn eat_whitespace() -> bool {
        // If the next character is a newline, and the last token ends with a
        // newline and whitespace, clear out the whitespace.

        alt self.classify_token() {
          normal { false }
          standalone {
              if self.ch == '\r' { self.bump(); }
              self.bump();
              true
          }
          whitespace(s, pos) | newline_whitespace(s, pos) {
              if self.ch == '\r' { self.bump(); }
              self.bump();

              // Trim the whitespace from the last token.
              vec::pop(self.tokens);
              vec::push(self.tokens, text(str::slice(s, 0u, pos)));

              true
          }
        }
    }

    fn add_tag() {
        self.bump();

        let tag = self.otag + self.content + self.ctag;
        let content = self.content;
        let content_len = str::len(content);

        alt content[0] as char {
          '!' {
            // ignore comments
            self.eat_whitespace();
          }
          '&' {
            let name = str::slice(content, 1u, content_len);
            let name = self.check_content(name);
            let name = str::split_char_nonempty(name, '.');
            vec::push(self.tokens, utag(name, tag)); }
          '{' {
            if str::ends_with(content, "}") {
                let name = str::slice(content, 1u, content_len - 1u);
                let name = self.check_content(name);
                let name = str::split_char_nonempty(name, '.');
                vec::push(self.tokens, utag(name, tag));
            } else {
                log(error, self.content);
                log(error, content);
                fail "unbalanced \"{\" in tag";
            }
          }
          '#' {
            let newlined = self.eat_whitespace();

            let name = self.check_content(str::slice(content, 1u, content_len));
            let name = str::split_char_nonempty(name, '.');
            vec::push(self.tokens, incomplete_section(name, false, tag, newlined));
          }
          '^' {
            let newlined = self.eat_whitespace();

            let name = self.check_content(str::slice(content, 1u, content_len));
            let name = str::split_char_nonempty(name, '.');
            vec::push(self.tokens, incomplete_section(name, true, tag, newlined));
          }
          '/' {
            self.eat_whitespace();

            let name = self.check_content(str::slice(content, 1u, content_len));
            let name = str::split_char_nonempty(name, '.');
            let mut children = [];

            while true {
                if vec::len(self.tokens) == 0u {
                    fail "closing unopened section";
                }

                let last = vec::pop(self.tokens);

                alt last {
                  incomplete_section(section_name, inverted, osection, _) {
                    let children = vec::reversed(children);

                    // Collect all the children's sources.
                    let mut srcs = [];
                    vec::iter(children) { |child: token|
                        alt child {
                          text(s)
                          | etag(_, s)
                          | utag(_, s)
                          | partial(_, _, s) {
                            vec::push(srcs, s);
                          }
                          section(_, _, _, _, osection, src, csection, _) {
                            vec::push(srcs, osection);
                            vec::push(srcs, src);
                            vec::push(srcs, csection);
                          }
                          _ { fail; }
                        }
                    }

                    if section_name == name {
                        vec::push(
                            self.tokens,
                            section(
                                name,
                                inverted,
                                children,
                                self.otag,
                                osection,
                                str::concat(srcs),
                                tag,
                                self.ctag));
                        break;
                    } else {
                        fail "Unclosed section";
                    }
                  }
                  _ { vec::push(children, last); }
                }
            }
          }
          '>' { self.add_partial(content, tag); }
          '=' {
            self.eat_whitespace();

            if (content_len > 2u && str::ends_with(content, "=")) {
                let s = self.check_content(str::slice(content, 1u, content_len - 1u));
                let pos = str::find_from(s, 0u) { |c|
                    char::is_whitespace(c)
                };

                let pos = alt pos {
                  none { fail "invalid change delimiter tag content"; }
                  some(pos) { pos }
                };

                self.otag = str::slice(s, 0u, pos);
                self.otag_chars = str::chars(self.otag);

                let pos = str::find_from(s, pos) { |c|
                    !char::is_whitespace(c)
                };

                let pos = alt pos {
                  none { fail "invalid change delimiter tag content"; }
                  some(pos) { pos }
                };

                self.ctag = str::slice(s, pos, str::len(s));
                self.ctag_chars = str::chars(self.ctag);
            } else {
                fail "invalid change delimiter tag content";
            }
          }
          _ {
            let name = self.check_content(content);
            let name = str::split_char_nonempty(name, '.');
            vec::push(self.tokens, etag(name, tag));
          }
        }

        self.content = "";
    }

    fn add_partial(content: str, tag: str) {
        let token_class = self.classify_token();
        let indent = alt token_class {
          normal { "" }
          standalone {
            if self.ch == '\r' { self.bump(); }
            self.bump();
            ""
          }
          whitespace(s, pos) | newline_whitespace(s, pos) {
            if self.ch == '\r' { self.bump(); }
            self.bump();

            let ws = str::slice(s, pos, str::len(s));

            // Trim the whitespace from the last token.
            vec::pop(self.tokens);
            vec::push(self.tokens, text(str::slice(s, 0u, pos)));

            ws
          }
        };

        // We can't inline the tokens directly as we may have a recursive
        // partial. So instead, we'll cache the partials we used and look them
        // up later.
        let name = str::slice(content, 1u, str::len(content));
        let name = self.check_content(name);

        vec::push(self.tokens, partial(name, indent, tag));
        vec::push(self.partials, name);
    }

    fn not_otag() {
        let mut i = 0u;
        while i < self.tag_position {
            unsafe { str::push_char(self.content, self.otag_chars[i]) };
            i += 1u;
        }
    }

    fn not_ctag() {
        let mut i = 0u;
        while i < self.tag_position {
            unsafe { str::push_char(self.content, self.ctag_chars[i]) };
            i += 1u;
        }
    }

    fn check_content(content: str) -> str {
        let trimmed = str::trim(content);
        if str::len(trimmed) == 0u {
            fail "empty tag";
        }
        trimmed
    }
}

fn compile_helper(rdr: io::reader,
                  partials: partial_map,
                  otag: str,
                  ctag: str) -> [token] {
    let parser = {
        rdr: rdr,
        mut ch: rdr.read_char(),
        mut lookahead: none,
        mut line: 1u,
        mut col: 1u,
        mut content: "",
        mut state: parser::TEXT,
        mut otag: otag,
        mut ctag: ctag,
        mut otag_chars: str::chars(otag),
        mut ctag_chars: str::chars(ctag),
        mut tag_position: 0u,
        mut tokens: [],
        mut partials: [],
    };

    let (tokens, partial_names) = parser.parse();

    // Compile the partials if we haven't done so already.
    vec::iter(partial_names) { |name|
        let path = name + ".mustache";

        if !partials.contains_key(name) {
            // Insert a placeholder so we don't recurse off to infinity.
            partials.insert(name, []);

            alt io::file_reader(path) {
              err(e) {}
              ok(rdr) {
                partials.insert(name,
                                compile_helper(rdr, partials, "{{", "}}"));
              }
            }
        }
    }

    tokens
}

fn render(template: template, context: context) -> str {
    render_helper({
        tokens: template.tokens,
        partials: template.partials,
        stack: [map(context)],
        indent: ""
    })
}

fn render_helper(ctx: render_context) -> str {
    fn find(stack: [data], path: [str]) -> option<data> {
        // If we have an empty path, we just want the top value in our stack.
        if vec::is_empty(path) {
            ret alt vec::last_opt(stack) {
              none { none }
              some(value) { some(value) }
            };
        }

        // Otherwise, find the stack that has the first part of our path.
        let mut value = none;

        let mut i = vec::len(stack);
        while i > 0u {
            alt stack[i - 1u] {
              map(ctx) {
                alt ctx.find(path[0u]) {
                  some(v) { value = some(v); break; }
                  none {}
                }
                i -= 1u;
              }
              _ { fail #fmt("%? %?", stack, path) }
            }
        }

        // Walk the rest of the path to find our final value.
        let mut value = value;

        let mut i = 1u;
        let len = vec::len(path);

        while i < len {
            alt value {
              some(map(v)) { value = v.find(path[i]); }
              _ { break; }
            }
            i += 1u;
        }

        value
    }

    let output = vec::map(ctx.tokens) { |token|
        let res = alt token {
          text(value) { indent_lines(value, ctx.indent) }
          etag(path, _) {
            alt find(ctx.stack, path) {
              none { "" }
              some(value) { ctx.indent + render_etag(value, ctx) }
            }
          }
          utag(path, _) {
            alt find(ctx.stack, path) {
              none { "" }
              some(value) { ctx.indent + render_utag(value, ctx) }
            }
          }
          section(path, true, children, _, _, _, _, _) {
            let ctx = {
                tokens: children,
                partials: ctx.partials,
                stack: ctx.stack,
                indent: ctx.indent,
            };

            alt find(ctx.stack, path) {
              none { render_helper(ctx) }
              some(value) { render_inverted_section(value, ctx) }
            }
          }
          section(path, false, children, otag, _, src, _, ctag) {
            alt find(ctx.stack, path) {
              none { "" }
              some(value) {
                render_section(value, src, otag, ctag, {
                    tokens: children,
                    partials: ctx.partials,
                    stack: ctx.stack,
                    indent: ctx.indent,
                })
              }
            }
          }
          partial(name, ind, _) {
            alt ctx.partials.find(name) {
              none { "" }
              some(tokens) {
                render_helper({
                    tokens: tokens,
                    partials: ctx.partials,
                    stack: ctx.stack,
                    indent: ctx.indent + ind,
                })
              }
            }
          }
          _ { fail }
        };

        res
    };

    str::concat(output)
}

fn indent_lines(s: str, indent: str) -> str {
    if indent == "" {
        s
    } else {
        let mut res = "";
        let mut pos = 0u;
        let len = str::len(s);

        while pos < len {
            let line = alt str::find_char_from(s, '\n', pos) {
              none {
                  let line = str::slice(s, pos, len);
                  pos = len;
                  line
              }
              some(i) {
                let line = str::slice(s, pos, i + 1u);
                pos = i + 1u;
                line
              }
            };

            if str::char_at(line, 0u) != '\n' { res += indent; }
            res += line;
        }
        res
    }
}

fn render_etag(value: data, ctx: render_context) -> str {
    let mut escaped = "";
    str::chars_iter(render_utag(value, ctx)) { |c|
        alt c {
          '<' { escaped += "&lt;" }
          '>' { escaped += "&gt;" }
          '&' { escaped += "&amp;" }
          '"' { escaped += "&quot;" }
          '\'' { escaped += "&#39;" }
          _ { str::push_char(escaped, c); }
        }
    }
    escaped
}

fn render_utag(value: data, ctx: render_context) -> str {
    alt value {
      str(s) { s }
      fun(f) {
          // etags and utags use the default delimiter.
          render_fun(ctx, "", "{{", "}}", f)
      }
      _ { fail }
    }
}

fn render_inverted_section(value: data, ctx: render_context) -> str {
    alt value {
      bool(false) { render_helper(ctx) }
      vec(xs) if vec::is_empty(xs) { render_helper(ctx) }
      _ { "" }
    }
}

fn render_section(value: data,
                  src: str,
                  otag: str,
                  ctag: str,
                  ctx: render_context) -> str {
    alt value {
      bool(true) { render_helper(ctx) }
      bool(false) { "" }
      vec(vs) {
        str::concat(vec::map(vs) { |v|
            render_helper({
                tokens: ctx.tokens,
                partials: ctx.partials,
                stack: ctx.stack + [v],
                indent: ctx.indent,
            })
        })
      }
      map(_) {
        render_helper({
            tokens: ctx.tokens,
            partials: ctx.partials,
            stack: ctx.stack + [value],
            indent: ctx.indent,
        })
      }
      fun(f) { render_fun(ctx, src, otag, ctag, f) }
      _ { fail }
    }
}

fn render_fun(ctx: render_context,
              src: str,
              otag: str,
              ctag: str,
              f: fn(str) -> str) -> str {
    let tokens = io::with_str_reader(f(src)) { |rdr|
        compile_helper(rdr, ctx.partials, otag, ctag)
    };

    render_helper({
        tokens: tokens,
        partials: ctx.partials,
        stack: ctx.stack,
        indent: ctx.indent,
    })
}

#[cfg(test)]
mod tests {
    import std::json;

    #[test]
    fn test_compile_texts() {
        assert compile_str("hello world").tokens == [text("hello world")];
        assert compile_str("hello {world").tokens == [text("hello {world")];
        assert compile_str("hello world}").tokens == [text("hello world}")];
        assert compile_str("hello world}}").tokens == [text("hello world}}")];
    }

    #[test]
    fn test_compile_etags() {
        assert compile_str("{{ name }}").tokens == [
            etag(["name"], "{{ name }}")
        ];

        assert compile_str("before {{name}} after").tokens == [
            text("before "),
            etag(["name"], "{{name}}"),
            text(" after")
        ];

        assert compile_str("before {{name}}").tokens == [
            text("before "),
            etag(["name"], "{{name}}")
        ];

        assert compile_str("{{name}} after").tokens == [
            etag(["name"], "{{name}}"),
            text(" after")
        ];
    }

    #[test]
    fn test_compile_utags() {
        assert compile_str("{{{name}}}").tokens == [
            utag(["name"], "{{{name}}}")
        ];

        assert compile_str("before {{{name}}} after").tokens == [
            text("before "),
            utag(["name"], "{{{name}}}"),
            text(" after")
        ];

        assert compile_str("before {{{name}}}").tokens == [
            text("before "),
            utag(["name"], "{{{name}}}")
        ];

        assert compile_str("{{{name}}} after").tokens == [
            utag(["name"], "{{{name}}}"),
            text(" after")
        ];
    }

    #[test]
    fn test_compile_sections() {
        assert compile_str("{{# name}}{{/name}}").tokens == [
            section(
                ["name"],
                false,
                [],
                "{{",
                "{{# name}}",
                "",
                "{{/name}}",
                "}}"
            )
        ];

        assert compile_str("before {{^name}}{{/name}} after").tokens == [
            text("before "),
            section(
                ["name"],
                true,
                [],
                "{{",
                "{{^name}}",
                "",
                "{{/name}}",
                "}}"
            ),
            text(" after")
        ];

        assert compile_str("before {{#name}}{{/name}}").tokens == [
            text("before "),
            section(
                ["name"],
                false,
                [],
                "{{",
                "{{#name}}",
                "",
                "{{/name}}",
                "}}"
            )
        ];

        assert compile_str("{{#name}}{{/name}} after").tokens == [
            section(
                ["name"],
                false,
                [],
                "{{",
                "{{#name}}",
                "",
                "{{/name}}",
                "}}"
            ),
            text(" after")
        ];

        assert compile_str(
                "before {{#a}} 1 {{^b}} 2 {{/b}} {{/a}} after").tokens == [
            text("before "),
            section(
                ["a"],
                false,
                [
                    text(" 1 "),
                    section(
                        ["b"],
                        true,
                        [text(" 2 ")],
                        "{{",
                        "{{^b}}",
                        " 2 ",
                        "{{/b}}",
                        "}}"
                    ),
                    text(" ")
                ],
                "{{",
                "{{#a}}",
                " 1 {{^b}} 2 {{/b}} ",
                "{{/a}}",
                "}}"
            ),
            text(" after")
        ];
    }

    #[test]
    fn test_compile_partials() {
        assert compile_str("{{> test}}").tokens == [
            partial("test", "", "{{> test}}")
        ];

        assert compile_str("before {{>test}} after").tokens == [
            text("before "),
            partial("test", "", "{{>test}}"),
            text(" after")
        ];

        assert compile_str("before {{> test}}").tokens == [
            text("before "),
            partial("test", "", "{{> test}}")
        ];

        assert compile_str("{{>test}} after").tokens == [
            partial("test", "", "{{>test}}"),
            text(" after")
        ];
    }

    #[test]
    fn test_compile_delimiters() {
        assert compile_str("before {{=<% %>=}}<%name%> after").tokens == [
            text("before "),
            etag(["name"], "<%name%>"),
            text(" after")
        ];
    }

    #[test]
    fn test_render_texts() {
        let ctx = str_hash();
        ctx.insert("name", str("world"));

        assert render(compile_str("hello world"), ctx) == "hello world";
        assert render(compile_str("hello {world"), ctx) == "hello {world";
        assert render(compile_str("hello world}"), ctx) == "hello world}";
        assert render(compile_str("hello {world}"), ctx) == "hello {world}";
        assert render(compile_str("hello world}}"), ctx) == "hello world}}";
    }

    #[test]
    fn test_render_etags() {
        let ctx = str_hash();
        ctx.insert("name", str("world"));

        assert render(compile_str("hello {{name}}"), ctx) == "hello world";
    }

    #[test]
    fn test_render_utags() {
        let ctx = str_hash();
        ctx.insert("name", str("world"));

        assert render(compile_str("hello {{{name}}}"), ctx) == "hello world";
    }

    #[test]
    fn test_render_sections() {
        let ctx0 = str_hash();
        let template = compile_str("0{{#a}}1 {{n}} 3{{/a}}5");

        assert render(template, ctx0) == "05";

        ctx0.insert("a", vec([]));
        assert render(template, ctx0) == "05";

        let ctx1 = str_hash();
        ctx0.insert("a", vec([map(ctx1)]));

        assert render(template, ctx0) == "01  35";

        ctx1.insert("n", str("a"));
        assert render(template, ctx0) == "01 a 35";

        ctx0.insert("a", fun({|text|
            assert text == "1 {{n}} 3";
            "foo"
        }));
        assert render(template, ctx0) == "0foo5";
    }

    #[test]
    fn test_render_inverted_sections() {
        let template = "0{{^a}}1 3{{/a}}5";

        let ctx0 = str_hash();
        assert render_str(template, ctx0) == "01 35";

        ctx0.insert("a", vec([]));
        assert render_str(template, ctx0) == "01 35";

        let ctx1 = str_hash();
        ctx0.insert("a", vec([map(ctx1)]));
        assert render_str(template, ctx0) == "05";

        ctx1.insert("n", str("a"));
        assert render_str(template, ctx0) == "05";
    }

    #[test]
    fn test_render_partial() {
        let path = "base.mustache";

        let ctx0 = str_hash();
        assert render_file(path, ctx0) == "<h2>Names</h2>\n";

        ctx0.insert("names", vec([]));
        assert render_file(path, ctx0) == "<h2>Names</h2>\n";

        let ctx1 = str_hash();
        ctx0.insert("names", vec([map(ctx1)]));
        assert render_file(path, ctx0) ==
            "<h2>Names</h2>\n" +
            "  <strong></strong>\n\n";

        ctx1.insert("name", str("a"));
        assert render_file(path, ctx0) ==
            "<h2>Names</h2>\n" +
            "  <strong>a</strong>\n\n";

        let ctx2 = str_hash();
        ctx2.insert("name", str("<b>"));
        ctx0.insert("names", vec([map(ctx1), map(ctx2)]));
        assert render_file(path, ctx0) ==
            "<h2>Names</h2>\n" +
            "  <strong>a</strong>\n\n" +
            "  <strong>&lt;b&gt;</strong>\n\n";
    }

    fn parse_spec_tests(src: str) -> [json::json] {
        alt io::read_whole_file_str(src) {
          err(e) { fail e }
          ok(s) {
            alt json::from_str(s) {
              err(e) { fail #fmt("%s:%u:%u: %s", src, e.line, e.col, e.msg) }
              ok(json) {
                alt json {
                  json::dict(d) {
                    alt d.find("tests") {
                      some(json::list(tests)) { tests }
                      _ { fail #fmt("%s: tests key not a list", src) }
                    }
                  }
                  _ { fail #fmt("%s: JSON value not a map", src) }
                }
              }
            }
          }
        }
    }

    fn convert_json_map(map: hashmap<str, json::json>) -> context {
        let ctx = str_hash();
        map.items { |key, value| ctx.insert(key, convert_json(value)); };
        ctx
    }

    fn convert_json(value: json::json) -> data {
        alt value {
          json::num(n) {
            // We have to cheat and use %? because %f doesn't convert 3.3 to
            // 3.3.
            str(#fmt("%?", n))
          }
          json::string(s) { str(s) }
          json::boolean(b) { bool(b) }
          json::list(v) { vec(vec::map(v, convert_json)) }
          json::dict(d) { map(convert_json_map(d)) }
          _ { fail #fmt("%?", value) }
        }
    }

    fn write_partials(value: json::json) -> [str] {
        let mut files = [];

        alt value {
          json::dict(d) {
            d.items { |key, value|
                alt value {
                  json::string(s) {
                    let file = key + ".mustache";
                    alt io::file_writer(file, [io::create, io::truncate]) {
                      ok(wr) { vec::push(files, file); wr.write_str(s); }
                      err(e) { fail e; }
                    }
                  }
                  _ { fail; }
                }
            }
          }
          _ { fail; }
        }

        files
    }

    fn run_test(test: hashmap<str, json::json>, ctx: context) {
        let template = alt test.get("template") {
          json::string(s) { s }
          _ { fail }
        };

        let expected = alt test.get("expected") {
          json::string(s) { s }
          _ { fail }
        };

        let partials = alt test.find("partials") {
          some(value) { write_partials(value) }
          none { [] }
        };

        let compiled = compile_str(template);
        let result = render(compiled, ctx);

        if result != expected {
            fn to_list(x: json::json) -> json::json {
                alt x {
                  json::dict(d) {
                    let mut xs = [];
                    d.items { |k,v|
                        let k = json::string(k);
                        let v = to_list(v);
                        vec::push(xs, json::list([k, v]));
                    }
                    json::list(xs)
                  }
                  json::list(xs) { json::list(vec::map(xs, to_list)) }
                  _ { x }
                }
            }

            io::println(#fmt("desc:     %?", test.get("desc")));
            io::println(#fmt("context:  %?", to_list(test.get("data"))));
            io::println(#fmt("partials: %?", partials));
            io::println(#fmt("partials: %?", test.find("partials")));
            io::println("");
            io::println(#fmt("template:\n%?", template));
            io::println(#fmt("expected:\n%?", expected));
            io::println(#fmt("result:  \n%?", result));
            io::println("");
            io::println(#fmt("template:\n%s", template));
            io::println(#fmt("expected:\n%s", expected));
            io::println(#fmt("result:  \n%s", result));
            io::println("");
            io::println(#fmt("compiled:\n%?", compiled));
        }
        assert result == expected;

        vec::iter(partials) { |file| os::remove_file(file); }
    }

    fn run_tests(spec: str) {
        vec::iter(parse_spec_tests(spec)) { |json|
            let test =  alt json {
              json::dict(m) { m }
              _ { fail }
            };

            let ctx = alt test.get("data") {
              json::dict(m) { convert_json_map(m) }
              _ { fail }
            };

            run_test(test, ctx);
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

    #[test]
    fn test_spec_lambdas() {
        vec::iter(parse_spec_tests("spec/specs/~lambdas.json")) { |json|
            let test =  alt json {
              json::dict(m) { m }
              _ { fail }
            };

            // Replace the lambda with rust code.
            let ctx = alt test.get("data") {
              json::dict(m) { convert_json_map(m) }
              _ { fail }
            };

            let lambda = alt test.get("name") {
              json::string("Interpolation") {
                  { |_text| "world" }
              }
              json::string("Interpolation - Expansion") {
                  { |_text| "{{planet}}" }
              }
              json::string("Interpolation - Alternate Delimiters") {
                  { |_text| "|planet| => {{planet}}" }
              }
              json::string("Interpolation - Multiple Calls") {
                  let calls = @mut 0;
                  { |_text| *calls += 1; int::str(*calls) }
              }
              json::string("Escaping") {
                  { |_text| ">" }
              }
              json::string("Section") {
                  { |text| if text == "{{x}}" { "yes" } else { "no" } }
              }
              json::string("Section - Expansion") {
                  { |text: str| text + "{{planet}}" + text }
              }
              json::string("Section - Alternate Delimiters") {
                  { |text: str| text + "{{planet}} => |planet|" + text }
              }
              json::string("Section - Multiple Calls") {
                  { |text| "__" + text + "__" }
              }
              json::string("Inverted Section") {
                  { |_text| "" }
              }
              value { fail #fmt("%?", value) }
            };

            ctx.insert("lambda", fun(lambda));

            run_test(test, ctx);
        }
    }
}
