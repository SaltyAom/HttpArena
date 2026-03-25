const std = @import("std");
const mem = std.mem;
const blitz = @import("blitz");

// ── Dataset items (parsed at startup, serialized per-request) ───────
const DatasetItem = struct {
    id: i64,
    name: []const u8,
    category: []const u8,
    price: f64,
    quantity: i64,
    active: bool,
    tags_json: []const u8, // pre-serialized JSON array string
    rating_score: f64,
    rating_count: i64,
};

var dataset_items: []DatasetItem = &[_]DatasetItem{};
var dataset_large_items: []DatasetItem = &[_]DatasetItem{};

// Pre-built JSON body for /compression (spec allows pre-serialization, only gzip must be per-request)
var compression_json_body: []const u8 = "";

// ── Per-thread SQLite (thread-local for zero contention) ────────────
threadlocal var tls_db: ?blitz.SqliteDb = null;
threadlocal var tls_db_stmt: ?blitz.SqliteStatement = null;
var db_available: bool = false;

const StaticFile = struct {
    name: []const u8,
    response: []const u8,
};
var static_file_list: [64]StaticFile = undefined;
var static_file_count: usize = 0;

// ── Handlers ────────────────────────────────────────────────────────

fn handlePipeline(_: *blitz.Request, res: *blitz.Response) void {
    _ = res.rawResponse("HTTP/1.1 200 OK\r\nServer: blitz\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok");
}

fn handleBaseline(req: *blitz.Request, res: *blitz.Response) void {
    var sum: i64 = 0;
    if (req.query) |q| sum = parseQuerySum(q);
    if (req.method == .POST) {
        if (req.body) |body| {
            const trimmed = mem.trim(u8, body, " \t\r\n");
            sum += std.fmt.parseInt(i64, trimmed, 10) catch 0;
        }
    }
    var nb: [32]u8 = undefined;
    _ = res.textBuf(blitz.writeI64(&nb, sum));
}

fn handleBaseline2(req: *blitz.Request, res: *blitz.Response) void {
    var sum: i64 = 0;
    if (req.query) |q| sum = parseQuerySum(q);
    var nb: [32]u8 = undefined;
    _ = res.textBuf(blitz.writeI64(&nb, sum));
}

fn handleJson(_: *blitz.Request, res: *blitz.Response) void {
    // Per-request: iterate all items, compute total, serialize JSON
    const items = dataset_items;
    if (items.len == 0) {
        _ = res.json("{\"items\":[],\"count\":0}");
        return;
    }

    var buf: [65536]u8 = undefined;
    var pos: usize = 0;

    const prefix = "{\"items\":[";
    @memcpy(buf[pos .. pos + prefix.len], prefix);
    pos += prefix.len;

    for (items, 0..) |item, idx| {
        if (idx > 0) {
            buf[pos] = ',';
            pos += 1;
        }

        // Compute total per-request as required by spec
        const total = @round(item.price * @as(f64, @floatFromInt(item.quantity)) * 100.0) / 100.0;

        const written = std.fmt.bufPrint(buf[pos..], "{{\"id\":{d},\"name\":", .{item.id}) catch break;
        pos += written.len;

        pos = writeJsonString(&buf, pos, item.name);

        const cat_prefix = ",\"category\":";
        @memcpy(buf[pos .. pos + cat_prefix.len], cat_prefix);
        pos += cat_prefix.len;
        pos = writeJsonString(&buf, pos, item.category);

        const fields = std.fmt.bufPrint(buf[pos..], ",\"price\":", .{}) catch break;
        pos += fields.len;
        pos = writeFloatBuf(&buf, pos, item.price);

        const qty = std.fmt.bufPrint(buf[pos..], ",\"quantity\":{d},\"active\":{s},\"tags\":", .{
            item.quantity,
            if (item.active) "true" else "false",
        }) catch break;
        pos += qty.len;

        // tags — pre-serialized JSON array
        if (item.tags_json.len > 0) {
            if (pos + item.tags_json.len < buf.len) {
                @memcpy(buf[pos .. pos + item.tags_json.len], item.tags_json);
                pos += item.tags_json.len;
            }
        } else {
            const empty = "[]";
            @memcpy(buf[pos .. pos + empty.len], empty);
            pos += empty.len;
        }

        const rating = std.fmt.bufPrint(buf[pos..], ",\"rating\":{{\"score\":", .{}) catch break;
        pos += rating.len;
        pos = writeFloatBuf(&buf, pos, item.rating_score);

        const rcount = std.fmt.bufPrint(buf[pos..], ",\"count\":{d}}},\"total\":", .{item.rating_count}) catch break;
        pos += rcount.len;
        pos = writeFloatBuf(&buf, pos, total);

        buf[pos] = '}';
        pos += 1;
    }

    const suffix = std.fmt.bufPrint(buf[pos..], "],\"count\":{d}}}", .{items.len}) catch {
        _ = res.setStatus(.internal_server_error).text("Buffer overflow");
        return;
    };
    pos += suffix.len;

    _ = res.json(buf[0..pos]);
}

// Thread-local buffers for compression (avoids heap alloc + use-after-free with rawResponse)
threadlocal var tls_gzip_buf: [1048576]u8 = undefined;
threadlocal var tls_resp_buf: [1048576 + 256]u8 = undefined;

fn handleCompression(req: *blitz.Request, res: *blitz.Response) void {
    // Check if client accepts gzip
    if (req.headers.get("Accept-Encoding")) |ae| {
        if (mem.indexOf(u8, ae, "gzip") != null) {
            // Per-request gzip compression at level 1 (BEST_SPEED)
            if (compression_json_body.len == 0) {
                _ = res.setStatus(.internal_server_error).text("No data");
                return;
            }

            var fbs = std.io.fixedBufferStream(&tls_gzip_buf);
            var compressor = std.compress.gzip.compressor(fbs.writer(), .{
                .level = .fast,
            }) catch {
                _ = res.json(compression_json_body);
                return;
            };
            compressor.writer().writeAll(compression_json_body) catch {
                _ = res.json(compression_json_body);
                return;
            };
            compressor.finish() catch {
                _ = res.json(compression_json_body);
                return;
            };
            const gzip_data = fbs.getWritten();

            // Build raw HTTP response with gzip headers into thread-local buffer
            const header = std.fmt.bufPrint(&tls_resp_buf, "HTTP/1.1 200 OK\r\nServer: blitz\r\nContent-Type: application/json\r\nContent-Encoding: gzip\r\nVary: Accept-Encoding\r\nContent-Length: {d}\r\n\r\n", .{gzip_data.len}) catch {
                _ = res.json(compression_json_body);
                return;
            };
            @memcpy(tls_resp_buf[header.len .. header.len + gzip_data.len], gzip_data);
            _ = res.rawResponse(tls_resp_buf[0 .. header.len + gzip_data.len]);
            return;
        }
    }
    // Fallback: uncompressed JSON
    _ = res.json(compression_json_body);
}

fn handleUpload(req: *blitz.Request, res: *blitz.Response) void {
    // Must actually read the body — spec requires reading the entire request body
    if (req.body) |body| {
        var nb: [32]u8 = undefined;
        _ = res.textBuf(blitz.writeUsize(&nb, body.len));
    } else {
        _ = res.text("0");
    }
}

fn handleWsUpgrade(req: *blitz.Request, res: *blitz.Response) void {
    if (!blitz.websocket.isUpgradeRequest(req)) {
        _ = res.text("WebSocket endpoint");
        return;
    }
    res.ws_upgraded = true;
}

fn handleDb(req: *blitz.Request, res: *blitz.Response) void {
    if (!db_available) {
        _ = res.setStatus(.internal_server_error).text("DB not available");
        return;
    }

    // Parse query params: ?min=10&max=50
    var min_price: f64 = 10.0;
    var max_price: f64 = 50.0;
    if (req.query) |q| {
        var it = mem.splitScalar(u8, q, '&');
        while (it.next()) |pair| {
            if (mem.indexOfScalar(u8, pair, '=')) |eq| {
                const key = pair[0..eq];
                const val = pair[eq + 1 ..];
                if (mem.eql(u8, key, "min")) {
                    min_price = std.fmt.parseFloat(f64, val) catch 10.0;
                } else if (mem.eql(u8, key, "max")) {
                    max_price = std.fmt.parseFloat(f64, val) catch 50.0;
                }
            }
        }
    }

    // Open per-thread DB connection + prepare statement (lazy init)
    if (tls_db == null) {
        tls_db = blitz.SqliteDb.open("/data/benchmark.db", .{ .readonly = true, .mmap_size = 64 * 1024 * 1024 }) catch {
            _ = res.setStatus(.internal_server_error).text("DB open failed");
            return;
        };
        tls_db_stmt = tls_db.?.prepare("SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ?1 AND ?2 LIMIT 50") catch {
            _ = res.setStatus(.internal_server_error).text("Prepare failed");
            return;
        };
    }

    var stmt = &(tls_db_stmt.?);
    stmt.reset();
    stmt.bindDouble(1, min_price) catch {
        _ = res.setStatus(.internal_server_error).text("Bind failed");
        return;
    };
    stmt.bindDouble(2, max_price) catch {
        _ = res.setStatus(.internal_server_error).text("Bind failed");
        return;
    };

    // Build JSON response into stack buffer
    var buf: [65536]u8 = undefined;
    var pos: usize = 0;

    const prefix = "{\"items\":[";
    @memcpy(buf[pos .. pos + prefix.len], prefix);
    pos += prefix.len;

    var count: usize = 0;
    while (true) {
        const has_row = stmt.step() catch break;
        if (!has_row) break;

        if (count > 0) {
            buf[pos] = ',';
            pos += 1;
        }

        const id = stmt.columnInt(0);
        const name = stmt.columnText(1);
        const category = stmt.columnText(2);
        const price = stmt.columnDouble(3);
        const quantity = stmt.columnInt(4);
        const active = stmt.columnInt(5);
        const tags_raw = stmt.columnText(6);
        const rating_score = stmt.columnDouble(7);
        const rating_count = stmt.columnInt(8);

        const written = std.fmt.bufPrint(buf[pos..], "{{\"id\":{d},\"name\":", .{id}) catch break;
        pos += written.len;

        pos = writeJsonString(&buf, pos, name);

        const cat_prefix = ",\"category\":";
        @memcpy(buf[pos .. pos + cat_prefix.len], cat_prefix);
        pos += cat_prefix.len;
        pos = writeJsonString(&buf, pos, category);

        const price_written = std.fmt.bufPrint(buf[pos..], ",\"price\":{d:.2},\"quantity\":{d},\"active\":{s},\"tags\":", .{
            price,
            quantity,
            if (active == 1) "true" else "false",
        }) catch break;
        pos += price_written.len;

        if (tags_raw.len > 0) {
            if (pos + tags_raw.len < buf.len) {
                @memcpy(buf[pos .. pos + tags_raw.len], tags_raw);
                pos += tags_raw.len;
            }
        } else {
            const empty = "[]";
            @memcpy(buf[pos .. pos + empty.len], empty);
            pos += empty.len;
        }

        const rating_written = std.fmt.bufPrint(buf[pos..], ",\"rating\":{{\"score\":{d:.1},\"count\":{d}}}}}", .{
            rating_score,
            rating_count,
        }) catch break;
        pos += rating_written.len;

        count += 1;
    }

    const suffix_written = std.fmt.bufPrint(buf[pos..], "],\"count\":{d}}}", .{count}) catch {
        _ = res.setStatus(.internal_server_error).text("Buffer overflow");
        return;
    };
    pos += suffix_written.len;

    _ = res.json(buf[0..pos]);
}

fn writeJsonString(buf: *[65536]u8, start: usize, s: []const u8) usize {
    var pos = start;
    buf[pos] = '"';
    pos += 1;
    for (s) |ch| {
        switch (ch) {
            '"' => {
                buf[pos] = '\\';
                buf[pos + 1] = '"';
                pos += 2;
            },
            '\\' => {
                buf[pos] = '\\';
                buf[pos + 1] = '\\';
                pos += 2;
            },
            '\n' => {
                buf[pos] = '\\';
                buf[pos + 1] = 'n';
                pos += 2;
            },
            '\r' => {
                buf[pos] = '\\';
                buf[pos + 1] = 'r';
                pos += 2;
            },
            '\t' => {
                buf[pos] = '\\';
                buf[pos + 1] = 't';
                pos += 2;
            },
            else => {
                buf[pos] = ch;
                pos += 1;
            },
        }
        if (pos >= buf.len - 2) break;
    }
    buf[pos] = '"';
    pos += 1;
    return pos;
}

// Write float with 2 decimal places into a fixed buffer at given position
fn writeFloatBuf(buf: *[65536]u8, start: usize, f: f64) usize {
    var pos = start;
    const rounded = @round(f * 100.0) / 100.0;
    const int_part: i64 = @intFromFloat(rounded);
    const frac = @abs(rounded - @as(f64, @floatFromInt(int_part)));
    const frac_int: u64 = @intFromFloat(@round(frac * 100.0));
    if (frac_int == 0) {
        const s = std.fmt.bufPrint(buf[pos..], "{d}.0", .{int_part}) catch return pos;
        pos += s.len;
    } else if (frac_int % 10 == 0) {
        const s = std.fmt.bufPrint(buf[pos..], "{d}.{d}", .{ int_part, frac_int / 10 }) catch return pos;
        pos += s.len;
    } else {
        const s = std.fmt.bufPrint(buf[pos..], "{d}.{d:0>2}", .{ int_part, frac_int }) catch return pos;
        pos += s.len;
    }
    return pos;
}

// Overloaded writeFloatBuf for larger buffers (compression handler)
fn writeFloatBufLarge(buf: []u8, start: usize, f: f64) usize {
    var pos = start;
    const rounded = @round(f * 100.0) / 100.0;
    const int_part: i64 = @intFromFloat(rounded);
    const frac = @abs(rounded - @as(f64, @floatFromInt(int_part)));
    const frac_int: u64 = @intFromFloat(@round(frac * 100.0));
    if (frac_int == 0) {
        const s = std.fmt.bufPrint(buf[pos..], "{d}.0", .{int_part}) catch return pos;
        pos += s.len;
    } else if (frac_int % 10 == 0) {
        const s = std.fmt.bufPrint(buf[pos..], "{d}.{d}", .{ int_part, frac_int / 10 }) catch return pos;
        pos += s.len;
    } else {
        const s = std.fmt.bufPrint(buf[pos..], "{d}.{d:0>2}", .{ int_part, frac_int }) catch return pos;
        pos += s.len;
    }
    return pos;
}

fn handleStatic(req: *blitz.Request, res: *blitz.Response) void {
    const filepath = req.params.get("filepath") orelse {
        _ = res.setStatus(.not_found).text("Not Found");
        return;
    };
    for (0..static_file_count) |i| {
        if (mem.eql(u8, static_file_list[i].name, filepath)) {
            _ = res.rawResponse(static_file_list[i].response);
            return;
        }
    }
    _ = res.setStatus(.not_found).text("Not Found");
}

// ── Helpers ─────────────────────────────────────────────────────────

fn parseQuerySum(query: []const u8) i64 {
    var sum: i64 = 0;
    var it = mem.splitScalar(u8, query, '&');
    while (it.next()) |pair| {
        if (mem.indexOfScalar(u8, pair, '=')) |eq| {
            sum += std.fmt.parseInt(i64, pair[eq + 1 ..], 10) catch continue;
        }
    }
    return sum;
}

// ── Dataset loading ─────────────────────────────────────────────────

fn parseDatasetItems(path: []const u8) []DatasetItem {
    const alloc = std.heap.c_allocator;
    const file = std.fs.openFileAbsolute(path, .{}) catch return &[_]DatasetItem{};
    defer file.close();
    const raw = file.readToEndAlloc(alloc, 64 * 1024 * 1024) catch return &[_]DatasetItem{};
    defer alloc.free(raw);

    const parsed = std.json.parseFromSlice(std.json.Value, alloc, raw, .{}) catch return &[_]DatasetItem{};
    const json_items = parsed.value.array.items;

    const items = alloc.alloc(DatasetItem, json_items.len) catch return &[_]DatasetItem{};

    for (json_items, 0..) |item, i| {
        const obj = item.object;
        items[i] = .{
            .id = switch (obj.get("id").?) {
                .integer => |v| v,
                else => 0,
            },
            .name = alloc.dupe(u8, switch (obj.get("name").?) {
                .string => |s| s,
                else => "",
            }) catch "",
            .category = alloc.dupe(u8, switch (obj.get("category").?) {
                .string => |s| s,
                else => "",
            }) catch "",
            .price = switch (obj.get("price").?) {
                .float => |f| f,
                .integer => |v| @as(f64, @floatFromInt(v)),
                else => 0.0,
            },
            .quantity = switch (obj.get("quantity").?) {
                .integer => |v| v,
                else => 0,
            },
            .active = switch (obj.get("active").?) {
                .bool => |b| b,
                else => false,
            },
            .tags_json = blk: {
                // Serialize tags array to JSON string
                var tags_buf = std.ArrayList(u8).init(alloc);
                const tags_val = obj.get("tags") orelse {
                    break :blk alloc.dupe(u8, "[]") catch "[]";
                };
                writeJsonValueToList(&tags_buf, tags_val);
                break :blk tags_buf.toOwnedSlice() catch "[]";
            },
            .rating_score = blk: {
                const rating = obj.get("rating") orelse break :blk 0.0;
                switch (rating) {
                    .object => |robj| {
                        const score = robj.get("score") orelse break :blk 0.0;
                        break :blk switch (score) {
                            .float => |f| f,
                            .integer => |v| @as(f64, @floatFromInt(v)),
                            else => 0.0,
                        };
                    },
                    else => break :blk 0.0,
                }
            },
            .rating_count = blk: {
                const rating = obj.get("rating") orelse break :blk 0;
                switch (rating) {
                    .object => |robj| {
                        const count_val = robj.get("count") orelse break :blk 0;
                        break :blk switch (count_val) {
                            .integer => |v| v,
                            else => 0,
                        };
                    },
                    else => break :blk 0,
                }
            },
        };
    }

    return items;
}

fn writeJsonValueToList(out: *std.ArrayList(u8), val: std.json.Value) void {
    switch (val) {
        .string => |s| {
            out.append('"') catch return;
            for (s) |ch| switch (ch) {
                '"' => out.appendSlice("\\\"") catch return,
                '\\' => out.appendSlice("\\\\") catch return,
                '\n' => out.appendSlice("\\n") catch return,
                '\r' => out.appendSlice("\\r") catch return,
                '\t' => out.appendSlice("\\t") catch return,
                else => out.append(ch) catch return,
            };
            out.append('"') catch return;
        },
        .integer => |i| {
            var buf: [32]u8 = undefined;
            const s = std.fmt.bufPrint(&buf, "{d}", .{i}) catch return;
            out.appendSlice(s) catch return;
        },
        .float => |f| writeFloatToList(out, f),
        .bool => |b| out.appendSlice(if (b) "true" else "false") catch return,
        .null => out.appendSlice("null") catch return,
        .array => |arr| {
            out.append('[') catch return;
            for (arr.items, 0..) |item, idx| {
                if (idx > 0) out.append(',') catch {};
                writeJsonValueToList(out, item);
            }
            out.append(']') catch return;
        },
        .object => |obj| {
            out.append('{') catch return;
            var first = true;
            var it = obj.iterator();
            while (it.next()) |entry| {
                if (!first) out.append(',') catch {};
                first = false;
                out.append('"') catch {};
                out.appendSlice(entry.key_ptr.*) catch {};
                out.appendSlice("\":") catch {};
                writeJsonValueToList(out, entry.value_ptr.*);
            }
            out.append('}') catch return;
        },
        else => {},
    }
}

fn writeFloatToList(out: *std.ArrayList(u8), f: f64) void {
    var buf: [64]u8 = undefined;
    const rounded = @round(f * 100.0) / 100.0;
    const int_part: i64 = @intFromFloat(rounded);
    const frac = @abs(rounded - @as(f64, @floatFromInt(int_part)));
    const frac_int: u64 = @intFromFloat(@round(frac * 100.0));
    if (frac_int == 0) {
        const s = std.fmt.bufPrint(&buf, "{d}.0", .{int_part}) catch return;
        out.appendSlice(s) catch return;
    } else if (frac_int % 10 == 0) {
        const s = std.fmt.bufPrint(&buf, "{d}.{d}", .{ int_part, frac_int / 10 }) catch return;
        out.appendSlice(s) catch return;
    } else {
        const s = std.fmt.bufPrint(&buf, "{d}.{d:0>2}", .{ int_part, frac_int }) catch return;
        out.appendSlice(s) catch return;
    }
}

// Pre-build the JSON body for compression endpoint (spec allows this — only gzip must be per-request)
fn buildCompressionJsonBody(items: []const DatasetItem) []const u8 {
    const alloc = std.heap.c_allocator;
    var json_buf = std.ArrayList(u8).init(alloc);
    json_buf.appendSlice("{\"items\":[") catch return "";

    for (items, 0..) |item, idx| {
        if (idx > 0) json_buf.append(',') catch {};

        const total = @round(item.price * @as(f64, @floatFromInt(item.quantity)) * 100.0) / 100.0;

        json_buf.appendSlice("{\"id\":") catch {};
        var id_buf: [32]u8 = undefined;
        const id_s = std.fmt.bufPrint(&id_buf, "{d}", .{item.id}) catch continue;
        json_buf.appendSlice(id_s) catch {};

        json_buf.appendSlice(",\"name\":") catch {};
        writeJsonStringToList(&json_buf, item.name);

        json_buf.appendSlice(",\"category\":") catch {};
        writeJsonStringToList(&json_buf, item.category);

        json_buf.appendSlice(",\"price\":") catch {};
        writeFloatToList(&json_buf, item.price);

        var qty_buf: [32]u8 = undefined;
        const qty_s = std.fmt.bufPrint(&qty_buf, ",\"quantity\":{d},\"active\":{s},\"tags\":", .{
            item.quantity,
            if (item.active) "true" else "false",
        }) catch continue;
        json_buf.appendSlice(qty_s) catch {};

        json_buf.appendSlice(item.tags_json) catch {};

        json_buf.appendSlice(",\"rating\":{\"score\":") catch {};
        writeFloatToList(&json_buf, item.rating_score);
        var rc_buf: [32]u8 = undefined;
        const rc_s = std.fmt.bufPrint(&rc_buf, ",\"count\":{d}}},\"total\":", .{item.rating_count}) catch continue;
        json_buf.appendSlice(rc_s) catch {};
        writeFloatToList(&json_buf, total);

        json_buf.append('}') catch {};
    }

    var count_buf: [32]u8 = undefined;
    json_buf.appendSlice("],\"count\":") catch {};
    json_buf.appendSlice(blitz.writeUsize(&count_buf, items.len)) catch {};
    json_buf.append('}') catch {};

    return json_buf.toOwnedSlice() catch "";
}

fn writeJsonStringToList(out: *std.ArrayList(u8), s: []const u8) void {
    out.append('"') catch return;
    for (s) |ch| switch (ch) {
        '"' => out.appendSlice("\\\"") catch return,
        '\\' => out.appendSlice("\\\\") catch return,
        '\n' => out.appendSlice("\\n") catch return,
        '\r' => out.appendSlice("\\r") catch return,
        '\t' => out.appendSlice("\\t") catch return,
        else => out.append(ch) catch return,
    };
    out.append('"') catch return;
}

// ── Static files ────────────────────────────────────────────────────

fn loadStaticFiles() void {
    const alloc = std.heap.c_allocator;
    var dir = std.fs.openDirAbsolute("/data/static", .{ .iterate = true }) catch return;
    defer dir.close();

    var iter = dir.iterate();
    while (iter.next() catch null) |entry| {
        if (entry.kind != .file) continue;
        if (static_file_count >= 64) break;

        const file = dir.openFile(entry.name, .{}) catch continue;
        defer file.close();
        const data = file.readToEndAlloc(alloc, 1024 * 1024) catch continue;
        const name = alloc.dupe(u8, entry.name) catch continue;
        const ct = getContentType(entry.name);

        var resp = std.ArrayList(u8).init(alloc);
        var cl_buf: [32]u8 = undefined;
        resp.appendSlice("HTTP/1.1 200 OK\r\nServer: blitz\r\nContent-Type: ") catch continue;
        resp.appendSlice(ct) catch continue;
        resp.appendSlice("\r\nContent-Length: ") catch continue;
        resp.appendSlice(blitz.writeUsize(&cl_buf, data.len)) catch continue;
        resp.appendSlice("\r\n\r\n") catch continue;
        resp.appendSlice(data) catch continue;

        static_file_list[static_file_count] = .{
            .name = name,
            .response = resp.toOwnedSlice() catch continue,
        };
        static_file_count += 1;
    }
}

fn getContentType(name: []const u8) []const u8 {
    if (mem.endsWith(u8, name, ".css")) return "text/css";
    if (mem.endsWith(u8, name, ".js")) return "application/javascript";
    if (mem.endsWith(u8, name, ".html")) return "text/html";
    if (mem.endsWith(u8, name, ".woff2")) return "font/woff2";
    if (mem.endsWith(u8, name, ".svg")) return "image/svg+xml";
    if (mem.endsWith(u8, name, ".webp")) return "image/webp";
    if (mem.endsWith(u8, name, ".json")) return "application/json";
    return "application/octet-stream";
}

// ── Main ────────────────────────────────────────────────────────────

pub fn main() !void {
    // Parse dataset items into structured data
    dataset_items = parseDatasetItems("/data/dataset.json");
    dataset_large_items = parseDatasetItems("/data/dataset-large.json");

    // Pre-build JSON body for compression endpoint (spec allows pre-serialization)
    compression_json_body = buildCompressionJsonBody(dataset_large_items);

    loadStaticFiles();

    // Check if benchmark.db exists for /db endpoint
    if (std.fs.openFileAbsolute("/data/benchmark.db", .{})) |f| {
        f.close();
        db_available = true;
    } else |_| {
        db_available = false;
    }

    // Set up router
    const alloc = std.heap.c_allocator;
    var router = blitz.Router.init(alloc);

    router.get("/pipeline", handlePipeline);
    router.get("/baseline11", handleBaseline);
    router.post("/baseline11", handleBaseline);
    router.get("/baseline2", handleBaseline2);
    router.get("/json", handleJson);
    router.get("/compression", handleCompression);
    router.post("/upload", handleUpload);
    router.get("/ws", handleWsUpgrade);
    router.get("/db", handleDb);
    router.get("/static/*filepath", handleStatic);

    // Check if io_uring backend is requested
    const use_uring = if (std.posix.getenv("BLITZ_URING")) |val| mem.eql(u8, val, "1") else false;

    if (use_uring) {
        var uring_server = blitz.UringServer.init(&router, .{
            .port = 8080,
            .compression = false,
        });
        uring_server.listen() catch {
            _ = std.posix.write(2, "uring: init failed, falling back to epoll\n") catch {};
            var server = blitz.Server.init(&router, .{
                .port = 8080,
                .keep_alive_timeout = 0,
                .compression = false,
            });
            try server.listen();
            return;
        };
    } else {
        var server = blitz.Server.init(&router, .{
            .port = 8080,
            .keep_alive_timeout = 0,
            .compression = false,
        });
        try server.listen();
    }
}
