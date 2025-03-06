const std = @import("std");

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
    var client = std.http.Client{ .allocator = allocator };
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
    var parsed = try std.json.parseFromSlice(
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

fn printJson(j: *std.json.Value, allocator: std.mem.Allocator) !void {
    // Convert JSON to a string and print it
    var json_string = std.ArrayList(u8).init(allocator);
    try std.json.stringify(j, .{}, json_string.writer());
    std.debug.print("Received JSON Response: {s}\n", .{json_string.items});
}
