#ifndef SHA1_H
#define SHA1_H

#define SHA1_DIGEST_SIZE 20

typedef uint8_t BYTE;
typedef uint32_t WORD;

struct SHA1_CTX {
    BYTE data[64];
    WORD datalen;
    unsigned long bitlen;
    WORD state[5];
    WORD k[4];
};

void metal_sha1_init(thread SHA1_CTX *);

#endif /* SHA1_H */
