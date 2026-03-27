#include <metal_stdlib>
using namespace metal;

// Must match Rust Uniforms struct layout.
struct Uniforms {
    uint grid_width;
    uint grid_height;
    float cell_width;
    float cell_height;
};

// --- Compute kernel: Game of Life update ---

kernel void update_cells(
    device const uint8_t* src [[buffer(0)]],
    device uint8_t* dst       [[buffer(1)]],
    constant Uniforms& uniforms [[buffer(2)]],
    uint2 gid [[thread_position_in_grid]])
{
    uint w = uniforms.grid_width;
    uint h = uniforms.grid_height;

    if (gid.x >= w || gid.y >= h) return;

    uint idx = gid.y * w + gid.x;
    uint8_t cell = src[idx];

    // Count alive (==255) neighbors with toroidal wrapping.
    uint8_t neighbors = 0;
    for (int dy = -1; dy <= 1; dy++) {
        for (int dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) continue;
            uint nx = (gid.x + uint(dx + int(w))) % w;
            uint ny = (gid.y + uint(dy + int(h))) % h;
            if (src[ny * w + nx] == 255) {
                neighbors++;
            }
        }
    }

    uint8_t result;
    if (cell == 255) {
        // Alive: survive with 2 or 3, otherwise begin dying
        result = (neighbors == 2 || neighbors == 3) ? 255 : 192;
    } else if (cell == 0) {
        // Dead: birth with exactly 3
        result = (neighbors == 3) ? 255 : 0;
    } else {
        // Dying (1–254): rebirth with 3, otherwise fast fade
        result = (neighbors == 3) ? 255 : uint8_t(max(int(cell) - 64, 0));
    }

    dst[idx] = result;
}

// --- Vertex shader: full-screen quad from vertex_id ---

struct VertexOut {
    float4 position [[position]];
    float2 uv;
};

vertex VertexOut fullscreen_quad_vertex(uint vid [[vertex_id]]) {
    // Two triangles forming a full-screen quad.
    // Triangle 0: (0,1,2), Triangle 1: (2,1,3)
    float2 positions[6] = {
        float2(-1.0, -1.0), // bottom-left
        float2( 1.0, -1.0), // bottom-right
        float2(-1.0,  1.0), // top-left
        float2(-1.0,  1.0), // top-left
        float2( 1.0, -1.0), // bottom-right
        float2( 1.0,  1.0), // top-right
    };

    float2 uvs[6] = {
        float2(0.0, 1.0), // bottom-left  -> uv bottom
        float2(1.0, 1.0), // bottom-right -> uv bottom
        float2(0.0, 0.0), // top-left     -> uv top
        float2(0.0, 0.0), // top-left     -> uv top
        float2(1.0, 1.0), // bottom-right -> uv bottom
        float2(1.0, 0.0), // top-right    -> uv top
    };

    VertexOut out;
    out.position = float4(positions[vid], 0.0, 1.0);
    out.uv = uvs[vid];
    return out;
}

// --- Fragment shader: map grid cell to brightness ---

fragment float4 grid_fragment(
    VertexOut in [[stage_in]],
    device const uint8_t* grid [[buffer(0)]],
    constant Uniforms& uniforms [[buffer(1)]])
{
    uint gx = uint(in.uv.x * float(uniforms.grid_width));
    uint gy = uint(in.uv.y * float(uniforms.grid_height));

    // Clamp to grid bounds.
    gx = min(gx, uniforms.grid_width - 1);
    gy = min(gy, uniforms.grid_height - 1);

    uint idx = gy * uniforms.grid_width + gx;
    float brightness = float(grid[idx]) / 255.0;

    return float4(brightness, brightness, brightness, 1.0);
}
