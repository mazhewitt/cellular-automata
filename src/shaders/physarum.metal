#include <metal_stdlib>
using namespace metal;

// ── Fixed simulation parameters (Jones 2010) ───────────────────────────
// Must match PhysarumConfig::default() in src/physarum.rs.
constant float SENSOR_ANGLE   = M_PI_F / 4.0;   // 45 degrees
constant float SENSOR_DIST    = 9.0;             // pixels
constant float TURN_SPEED     = M_PI_F / 4.0;    // 45 degrees per step
constant float MOVE_SPEED     = 1.0;             // pixels per step
constant float DEPOSIT_AMOUNT = 5.0;
constant float DECAY_FACTOR   = 0.95;

// Must match Rust Uniforms struct layout.
struct Uniforms {
    uint grid_width;
    uint grid_height;
    float cell_width;
    float cell_height;
};

// ── Helpers ─────────────────────────────────────────────────────────────

/// Toroidal wrap for float coordinate.
inline float wrap(float val, float max_val) {
    return fmod(fmod(val, max_val) + max_val, max_val);
}

/// Sample trail value at (fx, fy) from a single species plane.
inline float sample_trail(device const float* plane, float fx, float fy,
                          uint w, uint h) {
    uint x = uint(wrap(fx, float(w)));
    uint y = uint(wrap(fy, float(h)));
    return plane[y * w + x];
}

// ── Compute kernel: agent step ──────────────────────────────────────────
// Each thread processes one agent.
// Reads trail_src (previous frame), writes agent positions in-place,
// and deposits trail into trail_dst.

kernel void agent_step(
    device float4* agents            [[buffer(0)]],
    device const float* trail_src    [[buffer(1)]],
    device float* trail_dst          [[buffer(2)]],
    constant Uniforms& uniforms      [[buffer(3)]],
    constant uint& num_agents        [[buffer(4)]],
    uint gid [[thread_position_in_grid]])
{
    if (gid >= num_agents) return;

    float4 agent = agents[gid];
    float x       = agent.x;
    float y       = agent.y;
    float heading  = agent.z;
    uint species   = uint(agent.w);

    uint w = uniforms.grid_width;
    uint h = uniforms.grid_height;
    uint plane_size = w * h;

    // Pointer to this agent's species plane in trail_src.
    device const float* src_plane = trail_src + species * plane_size;

    // ── Sense ──
    float left_angle  = heading - SENSOR_ANGLE;
    float right_angle = heading + SENSOR_ANGLE;

    float probe_l = sample_trail(src_plane,
        x + SENSOR_DIST * cos(left_angle),
        y + SENSOR_DIST * sin(left_angle), w, h);
    float probe_c = sample_trail(src_plane,
        x + SENSOR_DIST * cos(heading),
        y + SENSOR_DIST * sin(heading), w, h);
    float probe_r = sample_trail(src_plane,
        x + SENSOR_DIST * cos(right_angle),
        y + SENSOR_DIST * sin(right_angle), w, h);

    // ── Rotate ──
    float new_heading = heading;
    if (probe_c >= probe_l && probe_c >= probe_r) {
        // Centre highest or tied — no turn
    } else if (probe_l > probe_r) {
        new_heading = heading - TURN_SPEED;
    } else {
        new_heading = heading + TURN_SPEED;
    }

    // ── Move ──
    float nx = wrap(x + MOVE_SPEED * cos(new_heading), float(w));
    float ny = wrap(y + MOVE_SPEED * sin(new_heading), float(h));

    agents[gid] = float4(nx, ny, new_heading, float(species));

    // ── Deposit ──
    uint cx = uint(nx);
    uint cy = uint(ny);
    uint dst_idx = species * plane_size + cy * w + cx;
    // Atomic add to handle multiple agents depositing to the same cell.
    device atomic_float* dst_cell = (device atomic_float*)&trail_dst[dst_idx];
    atomic_fetch_add_explicit(dst_cell, DEPOSIT_AMOUNT, memory_order_relaxed);
}

// ── Compute kernel: diffuse + decay ─────────────────────────────────────
// Each thread processes one cell across all species planes.

kernel void diffuse_decay(
    device const float* trail_src    [[buffer(0)]],
    device float* trail_dst          [[buffer(1)]],
    constant Uniforms& uniforms      [[buffer(2)]],
    uint2 gid [[thread_position_in_grid]])
{
    uint w = uniforms.grid_width;
    uint h = uniforms.grid_height;

    if (gid.x >= w || gid.y >= h) return;

    uint plane_size = w * h;

    for (uint s = 0; s < 3; s++) {
        uint offset = s * plane_size;
        float sum = 0.0;
        for (int dy = -1; dy <= 1; dy++) {
            for (int dx = -1; dx <= 1; dx++) {
                uint nx = (gid.x + uint(dx + int(w))) % w;
                uint ny = (gid.y + uint(dy + int(h))) % h;
                sum += trail_src[offset + ny * w + nx];
            }
        }
        trail_dst[offset + gid.y * w + gid.x] = (sum / 9.0) * DECAY_FACTOR;
    }
}

// ── Vertex shader: full-screen quad ─────────────────────────────────────

struct VertexOut {
    float4 position [[position]];
    float2 uv;
};

vertex VertexOut fullscreen_quad_vertex(uint vid [[vertex_id]]) {
    float2 positions[6] = {
        float2(-1.0, -1.0),
        float2( 1.0, -1.0),
        float2(-1.0,  1.0),
        float2(-1.0,  1.0),
        float2( 1.0, -1.0),
        float2( 1.0,  1.0),
    };

    float2 uvs[6] = {
        float2(0.0, 1.0),
        float2(1.0, 1.0),
        float2(0.0, 0.0),
        float2(0.0, 0.0),
        float2(1.0, 1.0),
        float2(1.0, 0.0),
    };

    VertexOut out;
    out.position = float4(positions[vid], 0.0, 1.0);
    out.uv = uvs[vid];
    return out;
}

// ── Fragment shader: additive multi-species colour blend ────────────────
// Palette: species 0 = cyan, species 1 = magenta, species 2 = gold

fragment float4 physarum_fragment(
    VertexOut in [[stage_in]],
    device const float* trail [[buffer(0)]],
    constant Uniforms& uniforms [[buffer(1)]])
{
    uint w = uniforms.grid_width;
    uint h = uniforms.grid_height;
    uint plane_size = w * h;

    uint gx = min(uint(in.uv.x * float(w)), w - 1);
    uint gy = min(uint(in.uv.y * float(h)), h - 1);
    uint idx = gy * w + gx;

    float t0 = trail[idx];                     // species 0
    float t1 = trail[plane_size + idx];        // species 1
    float t2 = trail[2 * plane_size + idx];    // species 2

    // Exposure-based tone mapping — compress high trail values into [0,1]
    float exposure = 0.05;
    float m0 = 1.0 - exp(-exposure * t0);
    float m1 = 1.0 - exp(-exposure * t1);
    float m2 = 1.0 - exp(-exposure * t2);

    // Organic palette (warm cream / amber / sage)
    float3 cream = float3(0.95, 0.90, 0.80);
    float3 amber = float3(0.85, 0.60, 0.20);
    float3 sage  = float3(0.40, 0.70, 0.40);

    float3 colour = m0 * cream + m1 * amber + m2 * sage;
    colour = clamp(colour, 0.0, 1.0);

    return float4(colour, 1.0);
}
