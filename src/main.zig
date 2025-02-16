const std = @import("std");
const json = std.json;
const http = std.http;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();

    // 1. Create HTTP client
    var client = http.Client{ .allocator = allocator };
    defer client.deinit();

    // 2. Initialize server_header_buffer
    var server_header_buffer: [4096]u8 = undefined;

    // 3. Open a GET request to the Discord Gateway endpoint
    var req = try client.open(.GET, try std.Uri.parse("https://discord.com/api/v10/gateway"), .{
        .server_header_buffer = &server_header_buffer,
    });
    defer req.deinit();

    // 4. Send the request
    try req.send();
    try req.wait();

    // 5. Read the response
    var response = std.ArrayList(u8).init(allocator);
    defer response.deinit();
    try req.reader().readAllArrayList(&response, 1024 * 1024);

    // 6. Parse the JSON response
    var parsed = try json.parseFromSlice(json.Value, allocator, response.items, .{});
    defer parsed.deinit();

    // 7. Convert JSON to a string and print it
    var json_buffer: [1024]u8 = undefined; // Temporary buffer
    var fba = std.heap.FixedBufferAllocator.init(&json_buffer);
    var json_string = std.ArrayList(u8).init(fba.allocator());

    try json.stringify(parsed.value, .{}, json_string.writer());

    std.debug.print("Received JSON Response: {s}\n", .{json_string.items});
}
