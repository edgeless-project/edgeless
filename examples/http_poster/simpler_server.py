#!/usr/bin/env python3

from http.server import BaseHTTPRequestHandler, HTTPServer


class SimpleHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8")
        print(body)

        self.send_response(200)
        self.send_header("Content-Type", "text/plain")
        self.end_headers()
        self.wfile.write(b"OK\n")

    def log_message(self, format, *args):
        pass


if __name__ == "__main__":
    server_address = ("", 10000)  # listen on all interfaces, port 8080
    httpd = HTTPServer(server_address, SimpleHandler)
    httpd.serve_forever()
