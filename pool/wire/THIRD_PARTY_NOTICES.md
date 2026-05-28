# Third-Party Notices

The `ai_blaise_citus_pool_wire` crate is a Rust port of the message shapes
and codec semantics of the PostgreSQL v3 wire-protocol parser shipped in
`jackc/pgx` (`pgproto3`), MIT licensed. The Rust code in this crate is
original work, but the on-the-wire shapes, message-tag set, and
encode/decode call surface mirror the upstream Go implementation. Per
MIT's "in all copies or substantial portions" requirement, the upstream
copyright notice is reproduced below.

## jackc/pgx pgproto3 (MIT)

Upstream: https://github.com/jackc/pgx

```
Copyright (c) 2013-2021 Jack Christensen

MIT License

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```
