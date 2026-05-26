#!/usr/bin/env python3
'''Minimal OpenAI-compatible chat completion server for CI evidence.

Exposes /v1/chat/completions accepting the standard request shape and
returning a deterministic completion response. Used by the A10/A11 live
smokes to prove the end-to-end HTTP integration code path from the
companion SQL extension without burning external LLM provider credits.
'''
import json
import time
from http.server import BaseHTTPRequestHandler, HTTPServer


SCHEMA_HINTS = {
    'orders': ('SELECT tenant_id, amount_cents FROM orders WHERE tenant_id =  LIMIT 100', 'amount_cents'),
}


class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        return  # quiet

    def do_GET(self):
        if self.path == '/healthz':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')
            return
        self.send_response(404)
        self.end_headers()

    def do_POST(self):
        if self.path != '/v1/chat/completions':
            self.send_response(404)
            self.end_headers()
            return
        length = int(self.headers.get('Content-Length', '0'))
        raw = self.rfile.read(length)
        try:
            body = json.loads(raw or b'{}')
        except json.JSONDecodeError:
            self.send_response(400)
            self.end_headers()
            return

        messages = body.get('messages', [])
        user_msg = ''
        for m in reversed(messages):
            if m.get('role') == 'user':
                user_msg = m.get('content', '')
                break
        if 'sql' in user_msg.lower():
            content = 'SELECT amount_cents, tenant_id FROM orders WHERE tenant_id = $1 LIMIT 100'
        else:
            content = 'mock_response_for_' + user_msg[:30].strip()

        payload = {
            'id': 'mock-cmpl-' + str(int(time.time())),
            'object': 'chat.completion',
            'created': int(time.time()),
            'model': body.get('model', 'mock-llm-1.0'),
            'choices': [{
                'index': 0,
                'message': {'role': 'assistant', 'content': content},
                'finish_reason': 'stop',
            }],
            'usage': {
                'prompt_tokens': sum(len(m.get('content', '').split()) for m in messages),
                'completion_tokens': len(content.split()),
                'total_tokens': sum(len(m.get('content', '').split()) for m in messages) + len(content.split()),
            },
        }
        body_bytes = json.dumps(payload).encode('utf-8')
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body_bytes)))
        self.end_headers()
        self.wfile.write(body_bytes)


if __name__ == '__main__':
    import sys
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
    HTTPServer(('0.0.0.0', port), Handler).serve_forever()
