//
//  sh_damien_metalsha.metal
//  sh.damien.metalsha
//
//  Created by Damien Broka on 10/10/2023.
//

#include <metal_stdlib>
using namespace metal;

kernel void add_arrays(device const uint* inA,
                       device const uint* inB,
                       device uint* result,
                       uint index [[thread_position_in_grid]])
{
    result[index] = inA[index] + inB[index];
}
