const std = @import("std");
const json = std.json;
const http = std.http;
const Allocator = std.mem.Allocator;
const net = std.net;
const posix = std.posix;
const base64 = std.base64;

// TODO:
// - try connect to WSS endpoint and parse hello event
// - heartbeat loop (sending and parsing ACKs)
// - responding to heartbeats and triggering next heartbeat loop
// - send identify with intents and parse ready event
// - handle disconnects (no heartbeat ACK, disconnect request): resume if possible, re-connect from scratch otherwise

// global data to add:
// - caching WSS URL returned by GET /gateway/bot
// - caching data required for Resuming
//   - session_id and resume_gateway_url returned by Ready event (on successfully connecting)
//   - sequence number (s) from last event dispatch (opcode 0)
// - comptime connect query params (e.g. "?v=10&encoding=json")

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

    const token = try std.process.getEnvVarOwned(allocator, "DISCORD_BOT_TOKEN");
    defer allocator.free(token);

    // Create HTTP client
    var client = http.Client{ .allocator = allocator };
    defer client.deinit();

    // Initialize server_header_buffer
    var server_header_buffer: [4096]u8 = undefined;

    // Open a GET request to the Discord Gateway endpoint
    var req = try client.open(.GET, try std.Uri.parse("https://discord.com/api/v10/gateway/bot"), .{
        .server_header_buffer = &server_header_buffer,
        .headers = .{
            .authorization = .{ .override = try std.fmt.allocPrint(allocator, "Bot {s}", .{token}) },
        },
    });
    defer req.deinit();

    // Send the request
    try req.send();
    try req.wait();

    // Read the response
    var response = std.ArrayList(u8).init(allocator);
    defer response.deinit();
    try req.reader().readAllArrayList(&response, 256); // usually 135 chars

    // Parse the JSON response
    const GetGatewayBotResponse = struct { url: []const u8 };
    var parsed = try json.parseFromSlice(
        GetGatewayBotResponse,
        allocator,
        response.items,
        .{ .ignore_unknown_fields = true }, // ignore shards and session_start_limit for now
    );
    defer parsed.deinit();

    // try printJson(&parsed.value, allocator);

    const wss_url = parsed.value.url;

    if (wss_url.len == 0) {
        return std.debug.print("Error: WSS URL not found in response\n", .{});
    }

    std.debug.print("WSS URL: {s}\n", .{wss_url});

    // connect to WebSocket endpoint
}

// Here's the frame diagram from https://datatracker.ietf.org/doc/html/rfc6455:
//
//   0                   1                   2                   3
//   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//   +-+-+-+-+-------+-+-------------+-------------------------------+
//   |F|R|R|R| opcode|M| Payload len |    Extended payload length    |
//   |I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
//   |N|V|V|V|       |S|             |   (if payload len==126/127)   |
//   | |1|2|3|       |K|             |                               |
//   +-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
//   |     Extended payload length continued, if payload len == 127  |
//   + - - - - - - - - - - - - - - - +-------------------------------+
//   |                               |Masking-key, if MASK set to 1  |
//   +-------------------------------+-------------------------------+
//   | Masking-key (continued)       |          Payload Data         |
//   +-------------------------------- - - - - - - - - - - - - - - - +
//   :                     Payload Data continued ...                :
//   + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
//   |                     Payload Data continued ...                |
//   +---------------------------------------------------------------+

fn maskKey() [4]u8 {
    var m: [4]u8 = undefined;
    std.crypto.random.bytes(&m);
    return m;
}

test "echo" {
    const address = try std.net.Address.parseIp("127.0.0.1", 8765);
    const tpe: u32 = posix.SOCK.STREAM;
    const protocol = posix.IPPROTO.TCP;
    const s = try posix.socket(address.any.family, tpe, protocol);
    defer posix.close(s);

    try posix.connect(s, &address.any, address.getOsSockLen());

    // Generate a random WebSocket key
    var random_bytes: [16]u8 = undefined;
    std.crypto.random.bytes(&random_bytes);
    var ws_key: [24]u8 = undefined;
    _ = base64.standard.Encoder.encode(&ws_key, &random_bytes);

    // Construct the WebSocket upgrade request
    const request = try std.fmt.allocPrint(std.testing.allocator, "GET / HTTP/1.1\r\n" ++
        "Host: localhost:8765\r\n" ++
        "Upgrade: websocket\r\n" ++
        "Connection: Upgrade\r\n" ++
        "Sec-WebSocket-Key: {s}\r\n" ++
        "Sec-WebSocket-Version: 13\r\n" ++
        "\r\n", .{ws_key});
    defer std.testing.allocator.free(request);

    // Send the upgrade request
    _ = try posix.write(s, request);

    // Read the server's response
    var buffer: [1024]u8 = undefined;
    var bytes_read = try posix.read(s, &buffer);
    const response = buffer[0..bytes_read];

    // Verify we got a successful upgrade response
    try std.testing.expect(std.mem.startsWith(u8, response, "HTTP/1.1 101"));

    // After successful handshake, let's send a simple text message
    // WebSocket frame format:
    // - Byte 0: fin(1) + rsv(3) + opcode(4)   = 0x81 for text message
    // - Byte 1: mask(1) + payload length(7)   = 0x80 | length
    // - Bytes 2-5: masking key
    // - Remaining bytes: masked payload

    const text = "Hello WebSocket!";
    const frame_header = [_]u8{
        0x81, // Final frame, text message
        0x80 | @as(u8, text.len), // Masked, length
    };

    // Generate random mask key
    const mask_key = maskKey();

    // Mask the payload
    var masked_payload: [text.len]u8 = undefined;
    for (text, 0..) |byte, i| {
        masked_payload[i] = byte ^ mask_key[i % 4];
    }

    // Send frame header
    _ = try posix.write(s, &frame_header);
    // Send mask key
    _ = try posix.write(s, &mask_key);
    // Send masked payload
    _ = try posix.write(s, &masked_payload);

    // Now wait for response
    bytes_read = try posix.read(s, &buffer);

    // Check echo

    const echo = buffer[0..bytes_read];

    // NOTE: We're checking a sequence of bytes here but some bytes, like the
    // first one, encode multiple flags/data. We check a range of bits in the
    // byte by ANDing (&) it with a mask (a byte with 1's for the bits we want
    // to check). This zeroes out the bits we aren't checking.
    //
    // For example, if we wanted to check the first three bits of a byte:
    //
    // const byte: u8 = 0b10101100;
    // const mask: u8 = 0b11100000;
    // const expected: u8 = 0b10100000;
    // try std.testing.expect(byte & mask == expected); // PASSES

    // first byte of header
    try std.testing.expect(echo[0] & 0b10000000 != 0); // fin == 1, this frame is the whole message
    try std.testing.expect(echo[0] & 0b01110000 == 0); // rsvx == 0, not using reserved extension bits
    try std.testing.expect(echo[0] & 0b00001111 == 1); // opcode 1, text frame

    // second byte of header
    try std.testing.expect(echo[1] & 0b10000000 == 0); // mask == 0, server messages not masked
    try std.testing.expect(echo[1] & 0b01111111 == text.len); // response payload length same as payload sent

    // remaining bytes are the payload (messages from server are unmasked, so the key is omitted)
    const payload = echo[2 .. 2 + text.len];
    try std.testing.expectEqualStrings(text, payload);

    // Send close frame (optional but polite)
    const close_frame = [_]u8{
        0x88, // Final frame, close opcode
        0x80, // Masked, zero length
    } ++ maskKey();
    _ = try posix.write(s, &close_frame);
}

fn printJson(j: *json.Value, allocator: Allocator) !void {
    // Convert JSON to a string and print it
    var json_string = std.ArrayList(u8).init(allocator);
    try json.stringify(j, .{}, json_string.writer());
    std.debug.print("Received JSON Response: {s}\n", .{json_string.items});
}
