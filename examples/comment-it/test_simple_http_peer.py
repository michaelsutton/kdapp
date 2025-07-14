#!/usr/bin/env python3

# Simple HTTP peer test to verify port 8080 is available
import socket
import sys

def test_port(port):
    """Test if a port is available"""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)
        result = sock.connect_ex(('127.0.0.1', port))
        sock.close()
        return result == 0  # 0 means connection successful (port in use)
    except Exception as e:
        print(f"Error testing port {port}: {e}")
        return False

def start_simple_http_peer(port):
    """Start a simple HTTP peer for testing"""
    try:
        import http.server
        import socketserver
        
        with socketserver.TCPServer(("", port), http.server.SimpleHTTPRequestHandler) as httpd:
            print(f"✅ Simple HTTP peer started on port {port}")
            print(f"🌐 Test URL: http://localhost:{port}")
            print("Press Ctrl+C to stop")
            httpd.serve_forever()
    except Exception as e:
        print(f"❌ Failed to start HTTP peer on port {port}: {e}")
        return False

if __name__ == "__main__":
    port = 8080
    
    print(f"🔍 Testing port {port} availability...")
    
    if test_port(port):
        print(f"❌ Port {port} is already in use")
        print("Something else is running on this port")
        sys.exit(1)
    else:
        print(f"✅ Port {port} is available")
        print("Starting simple test HTTP peer...")
        start_simple_http_peer(port)