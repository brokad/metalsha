#include <metal_stdlib>
using namespace metal;

#include "digest.h"
#include "sha1.h"

#ifndef ROTLEFT
#define ROTLEFT(a,b) (((a) << (b)) | ((a) >> (32-(b))))
#endif

void metal_sha1_transform(thread SHA1_CTX *ctx)
{
    WORD a, b, c, d, e, i, j, t, m[80];
    thread BYTE *data = ctx->data;

    for (i = 0, j = 0; i < 16; ++i, j += 4)
        m[i] = (data[j] << 24) + (data[j + 1] << 16) + (data[j + 2] << 8) + (data[j + 3]);
    for ( ; i < 80; ++i) {
        m[i] = (m[i - 3] ^ m[i - 8] ^ m[i - 14] ^ m[i - 16]);
        m[i] = (m[i] << 1) | (m[i] >> 31);
    }

    a = ctx->state[0];
    b = ctx->state[1];
    c = ctx->state[2];
    d = ctx->state[3];
    e = ctx->state[4];

    for (i = 0; i < 20; ++i) {
        t = ROTLEFT(a, 5) + ((b & c) ^ (~b & d)) + e + ctx->k[0] + m[i];
        e = d;
        d = c;
        c = ROTLEFT(b, 30);
        b = a;
        a = t;
    }
    for ( ; i < 40; ++i) {
        t = ROTLEFT(a, 5) + (b ^ c ^ d) + e + ctx->k[1] + m[i];
        e = d;
        d = c;
        c = ROTLEFT(b, 30);
        b = a;
        a = t;
    }
    for ( ; i < 60; ++i) {
        t = ROTLEFT(a, 5) + ((b & c) ^ (b & d) ^ (c & d))  + e + ctx->k[2] + m[i];
        e = d;
        d = c;
        c = ROTLEFT(b, 30);
        b = a;
        a = t;
    }
    for ( ; i < 80; ++i) {
        t = ROTLEFT(a, 5) + (b ^ c ^ d) + e + ctx->k[3] + m[i];
        e = d;
        d = c;
        c = ROTLEFT(b, 30);
        b = a;
        a = t;
    }

    ctx->state[0] += a;
    ctx->state[1] += b;
    ctx->state[2] += c;
    ctx->state[3] += d;
    ctx->state[4] += e;
}

void metal_sha1_init(thread SHA1_CTX *ctx)
{
    ctx->datalen = 0;
    ctx->bitlen = 0;
    ctx->state[0] = 0x67452301;
    ctx->state[1] = 0xEFCDAB89;
    ctx->state[2] = 0x98BADCFE;
    ctx->state[3] = 0x10325476;
    ctx->state[4] = 0xc3d2e1f0;
    ctx->k[0] = 0x5A827999;
    ctx->k[1] = 0x6ED9EBA1;
    ctx->k[2] = 0x8F1BBCDC;
    ctx->k[3] = 0xCA62C1D6;
}

void metal_sha1_update(thread SHA1_CTX *ctx, 
                       device const BYTE *in,
                       uint64_t len)
{
    uint64_t i;
    
    for (i = 0; i < len; i++) {
        ctx->data[ctx->datalen] = in[i];
        ctx->datalen++;
        if (ctx->datalen == 64) {
            metal_sha1_transform(ctx);
            ctx->bitlen += 512;
            ctx->datalen = 0;
        }
    }
}

void metal_sha1_final(thread SHA1_CTX *ctx,
                      device BYTE *out)
{
    WORD i;

    i = ctx->datalen;

    // Pad whatever data is left in the buffer.
    if (ctx->datalen < 56) {
        ctx->data[i++] = 0x80;
        while (i < 56)
            ctx->data[i++] = 0x00;
    }
    else {
        ctx->data[i++] = 0x80;
        while (i < 64)
            ctx->data[i++] = 0x00;
        metal_sha1_transform(ctx);
        i = 0;
        while (i < 56)
            ctx->data[i++] = 0;
    }

    // Append to the padding the total message's length in bits and transform.
    ctx->bitlen += ctx->datalen * 8;
    ctx->data[63] = ctx->bitlen;
    ctx->data[62] = ctx->bitlen >> 8;
    ctx->data[61] = ctx->bitlen >> 16;
    ctx->data[60] = ctx->bitlen >> 24;
    ctx->data[59] = ctx->bitlen >> 32;
    ctx->data[58] = ctx->bitlen >> 40;
    ctx->data[57] = ctx->bitlen >> 48;
    ctx->data[56] = ctx->bitlen >> 56;
    metal_sha1_transform(ctx);

    // Since this implementation uses little endian byte ordering and MD uses big endian,
    // reverse all the bytes when copying the final state to the output hash.
    for (i = 0; i < 4; ++i) {
        out[i]      = (ctx->state[0] >> (24 - i * 8)) & 0x000000ff;
        out[i + 4]  = (ctx->state[1] >> (24 - i * 8)) & 0x000000ff;
        out[i + 8]  = (ctx->state[2] >> (24 - i * 8)) & 0x000000ff;
        out[i + 12] = (ctx->state[3] >> (24 - i * 8)) & 0x000000ff;
        out[i + 16] = (ctx->state[4] >> (24 - i * 8)) & 0x000000ff;
    }
}

kernel void kernel_sha1_hash(device const BYTE *indata [[buffer(0)]],
                             device BYTE *outdata [[buffer(1)]],
                             device const DIGEST_ARGS *args [[buffer(2)]],
                             uint gid [[thread_position_in_grid]])
{
    thread SHA1_CTX ctx;
    uint64_t inlen = args->inlen;
    device const BYTE *in = indata + inlen * gid;
    device BYTE *out = outdata + SHA1_DIGEST_SIZE * gid;
    
    metal_sha1_init(&ctx);
    metal_sha1_update(&ctx, in, inlen);
    metal_sha1_final(&ctx, out);
}
