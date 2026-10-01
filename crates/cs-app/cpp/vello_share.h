#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// kind: 1 = D3D12 device, 2 = Vulkan device.
// Handles are passed as integer bits so COM pointers and Vulkan handles share one ABI.

// 1 when the next begin() must replace the shared textures.
int cs_vello_share_reset_needed(int kind, uint64_t device, int width, int height);

// Create or reuse the two shared frames. Writes the slot Qt is not sampling.
// Returns 1 on success.
int cs_vello_share_begin(int kind, uint64_t device, uint64_t queue, uint64_t instance,
                         uint64_t physical, unsigned family, int width, int height,
                         int *out_slot, int *out_epoch);

// Extra reference to slot's native texture (ID3D12Resource or VkImage bits).
// The caller owns that reference. 0 on failure.
uint64_t cs_vello_share_resource(int slot);

// The GPU has finished writing `slot`. Qt may sample it.
void cs_vello_share_commit(int slot);

int cs_vello_share_has_base(int width, int height);

// Slot to patch, the current picture, and whether that picture must be copied first.
// Returns 1 when a frame of the right size exists.
int cs_vello_share_patch_dest(int *out_dst, int *out_src, int *out_copy_full);

int cs_vello_share_blocked(void);
void cs_vello_share_block(void);
uint64_t cs_vello_share_generation(void);
void cs_vello_share_release(void);

// Render thread. Wraps the published frame as a QSGTexture* for `window`.
// Null when zero-copy is not available; the caller uploads pixels instead.
void *cs_vello_share_adopt(void *window, int *out_w, int *out_h);

#ifdef __cplusplus
}
#endif
