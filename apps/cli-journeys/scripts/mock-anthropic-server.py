#!/usr/bin/env python3
"""
Mock Anthropic API server for testing hwledger-verify without real API calls.
Responds to describe and judge endpoints with canned responses matching journey intents.
"""

import json
import sys
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse
import base64

PORT = 8765

# Canned responses indexed by journey tape name
CANNED_RESPONSES = {
    "cli-ingest-error": {
        "describe": {
            "type": "text",
            "text": "The screenshot shows a terminal with the command 'hwledger ingest gguf:///tmp/nonexistent.gguf' being executed. Error message displayed indicates the file path does not exist."
        },
        "judge": {
            "type": "text",
            "text": json.dumps({
                "intent_match": "PASS",
                "confidence": 0.95,
                "observations": [
                    "Error handling displayed correctly",
                    "CLI gracefully handles missing GGUF file",
                    "User feedback is clear and actionable"
                ],
                "score": 0.92
            })
        }
    },
    "cli-plan-deepseek": {
        "describe": {
            "type": "text",
            "text": "The screenshot shows hwledger computing memory allocation for DeepSeek-V3 model. A formatted table displays attention kind (MLA), parameter counts, memory breakdown (weights, KV cache, activations, overhead), and total memory requirement in MB."
        },
        "judge": {
            "type": "text",
            "text": json.dumps({
                "intent_match": "PASS",
                "confidence": 0.98,
                "observations": [
                    "Memory calculation performed correctly",
                    "Table formatting is professional and readable",
                    "All expected fields present: weights, KV cache, activations, overhead",
                    "Context window (seq=2048) and user count (2) honored"
                ],
                "score": 0.95
            })
        }
    },
    "cli-plan-help": {
        "describe": {
            "type": "text",
            "text": "The screenshot displays the help text for the 'hwledger plan' subcommand, showing usage syntax, arguments, and available options including --seq, --users, --batch, and quantization settings."
        },
        "judge": {
            "type": "text",
            "text": json.dumps({
                "intent_match": "PASS",
                "confidence": 0.96,
                "observations": [
                    "Help text is complete and well-formatted",
                    "All major options documented",
                    "Default values clearly shown",
                    "Description is concise and actionable"
                ],
                "score": 0.93
            })
        }
    },
    "cli-probe-list": {
        "describe": {
            "type": "text",
            "text": "The screenshot shows the output of 'hwledger probe list --json'. JSON response contains schema version, timestamp, and an empty devices array (expected on non-GPU hardware)."
        },
        "judge": {
            "type": "text",
            "text": json.dumps({
                "intent_match": "PASS",
                "confidence": 0.94,
                "observations": [
                    "JSON output is valid and properly formatted",
                    "Schema version correctly reported",
                    "Command handles GPU absence gracefully",
                    "No error messages in output"
                ],
                "score": 0.91
            })
        }
    },
    "cli-probe-watch": {
        "describe": {
            "type": "text",
            "text": "The screenshot shows 'hwledger probe watch' monitoring GPU resources. Output displays periodic JSON updates with device status. Process terminated cleanly with Ctrl+C."
        },
        "judge": {
            "type": "text",
            "text": json.dumps({
                "intent_match": "PASS",
                "confidence": 0.93,
                "observations": [
                    "Watch mode operates without errors",
                    "Interval-based updates functioning",
                    "Clean shutdown on Ctrl+C",
                    "JSON format maintained throughout"
                ],
                "score": 0.90
            })
        }
    }
}


class MockAnthropicHandler(BaseHTTPRequestHandler):
    """HTTP request handler for mock Anthropic API."""

    def do_POST(self):
        """Handle POST requests to mock API endpoints."""
        path = urlparse(self.path).path

        # Read request body
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length).decode('utf-8')

        try:
            request_data = json.loads(body)
        except json.JSONDecodeError:
            self.send_error(400, "Invalid JSON")
            return

        # Route based on path
        if path == "/v1/messages":
            self._handle_messages(request_data)
        else:
            self.send_error(404, f"Unknown endpoint: {path}")

    def _handle_messages(self, request_data):
        """Handle /messages endpoint (describe and judge calls)."""
        # Extract journey ID from messages (expect base64-encoded image with filename hint)
        messages = request_data.get("messages", [])
        journey_id = None

        # Try to infer journey from message content
        for msg in messages:
            content = msg.get("content", [])
            for item in content:
                if item.get("type") == "text":
                    text = item.get("text", "")
                    # Look for journey mention in prompt
                    for key in CANNED_RESPONSES.keys():
                        if key in text.lower() or key.replace("-", "_") in text.lower():
                            journey_id = key
                            break

        # Default to first journey if not found
        if not journey_id:
            journey_id = "cli-plan-deepseek"

        # Determine response type based on system prompt
        system = request_data.get("system", "")
        response_type = "describe" if "describe" in system.lower() else "judge"

        # Get canned response
        canned = CANNED_RESPONSES.get(journey_id, CANNED_RESPONSES["cli-plan-deepseek"])
        response_content = canned.get(response_type, {"type": "text", "text": "Mock response"})

        # Build response
        response = {
            "id": f"msg_{journey_id}_{response_type}",
            "type": "message",
            "role": "assistant",
            "content": [response_content],
            "model": "claude-3-5-sonnet-20241022",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50
            }
        }

        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(response).encode('utf-8'))

    def log_message(self, format, *args):
        """Suppress default logging."""
        pass


def main():
    """Start the mock server."""
    server = HTTPServer(("127.0.0.1", PORT), MockAnthropicHandler)
    print(f"Mock Anthropic server listening on http://127.0.0.1:{PORT}")
    print("Press Ctrl+C to stop.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nServer stopped.")
        sys.exit(0)


if __name__ == "__main__":
    main()
