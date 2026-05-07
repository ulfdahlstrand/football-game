#!/usr/bin/env python3
"""
Match log server — tar emot POST /log från spelet och sparar till match-logs/.
Kör med:  python scripts/log-server.py
Loggar sparas som match-logs/YYYY-MM-DD_HH-MM-SS_<team0>-vs-<team1>.json
"""
import json
import os
import sys
from datetime import datetime
from http.server import BaseHTTPRequestHandler, HTTPServer

PORT = 8766
LOG_DIR = os.path.join(os.path.dirname(__file__), '..', 'match-logs')


class LogHandler(BaseHTTPRequestHandler):
    def do_OPTIONS(self):
        self.send_response(204)
        self._cors()
        self.end_headers()

    def do_POST(self):
        if self.path != '/log':
            self.send_response(404)
            self.end_headers()
            return

        length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(length)

        try:
            data = json.loads(body)
        except json.JSONDecodeError as e:
            self.send_response(400)
            self._cors()
            self.end_headers()
            self.wfile.write(f'Bad JSON: {e}'.encode())
            return

        os.makedirs(LOG_DIR, exist_ok=True)

        ts = datetime.now().strftime('%Y-%m-%d_%H-%M-%S')
        t0 = _slug(data.get('team0', 'lag0'))
        t1 = _slug(data.get('team1', 'lag1'))
        score = data.get('score', [0, 0])
        filename = f'{ts}_{t0}-{score[0]}-vs-{score[1]}-{t1}.json'
        path = os.path.join(LOG_DIR, filename)

        with open(path, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

        print(f'  ✓  {filename}  ({len(data.get("log", []))} events)')

        self.send_response(200)
        self._cors()
        self.end_headers()
        self.wfile.write(json.dumps({'saved': filename}).encode())

    def _cors(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.send_header('Access-Control-Allow-Methods', 'POST, OPTIONS')

    def log_message(self, fmt, *args):
        pass  # tysta standard-access-loggar


def _slug(name):
    return ''.join(c if c.isalnum() else '-' for c in str(name)).strip('-')[:24]


if __name__ == '__main__':
    os.makedirs(LOG_DIR, exist_ok=True)
    print(f'Match log server  →  http://localhost:{PORT}')
    print(f'Sparar till       →  match-logs/')
    print('Ctrl-C för att stänga\n')
    try:
        HTTPServer(('localhost', PORT), LogHandler).serve_forever()
    except KeyboardInterrupt:
        print('\nStänger.')
        sys.exit(0)
