#define _LARGEFILE64_SOURCE
#define _FILE_OFFSET_BITS 64
#define _CRT_SECURE_NO_WARNINGS

#include "zstd.h"
#include "HDiffPatch/file_for_patch.h"
#include "HDiffPatch/libHDiffPatch/HPatch/patch.h"
#include "HDiffPatch/decompress_plugin_demo.h"

#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#define kMinCacheSize (hpatch_kStreamCacheSize * 3)
#define kExtraCacheSize (1 << 21)

typedef struct
{
    unsigned char* buf;
} PatchState;

static hpatch_BOOL on_diff_info(sspatch_listener_t* listener,
                                const hpatch_singleCompressedDiffInfo* info,
                                hpatch_TDecompress** out_decompressPlugin,
                                unsigned char** out_temp_cache,
                                unsigned char** out_temp_cacheEnd)
{
    PatchState* state = (PatchState*)listener->import;

    if (info->compressType[0] == '\0')
        *out_decompressPlugin = NULL;
    else if (zstdDecompressPlugin.is_can_open(info->compressType))
        *out_decompressPlugin = &zstdDecompressPlugin;
    else
        return hpatch_FALSE;

    size_t need = (size_t)info->stepMemSize + kExtraCacheSize;
    if (need < (size_t)info->stepMemSize + kMinCacheSize)
        need = (size_t)info->stepMemSize + kMinCacheSize;

    state->buf = (unsigned char*)malloc(need);
    if (!state->buf)
        return hpatch_FALSE;

    *out_temp_cache = state->buf;
    *out_temp_cacheEnd = state->buf + need;
    return hpatch_TRUE;
}

static void on_patch_finish(sspatch_listener_t* listener,
                            unsigned char* temp_cache,
                            unsigned char* temp_cacheEnd)
{
    PatchState* state = (PatchState*)listener->import;
    if (state->buf)
    {
        free(state->buf);
        state->buf = NULL;
    }
}

static hpatch_BOOL execute_patch(const char* old_path,
                                 const char* patch_path,
                                 const char* out_path)
{
    hpatch_TFileStreamInput oldData;
    hpatch_TFileStreamInput diffData;
    hpatch_TFileStreamOutput outData;
    hpatch_BOOL result = hpatch_FALSE;

    hpatch_TFileStreamInput_init(&oldData);
    hpatch_TFileStreamInput_init(&diffData);
    hpatch_TFileStreamOutput_init(&outData);

    if (old_path && old_path[0] != '\0')
    {
        if (!hpatch_TFileStreamInput_open(&oldData, old_path))
            return hpatch_FALSE;
    }

    if (!hpatch_TFileStreamInput_open(&diffData, patch_path))
    {
        hpatch_TFileStreamInput_close(&oldData);
        return hpatch_FALSE;
    }

    if (!hpatch_TFileStreamOutput_open(&outData, out_path, (hpatch_StreamPos_t)(-1)))
        goto cleanup;

    hpatch_TFileStreamOutput_setRandomOut(&outData, hpatch_TRUE);

    hpatch_singleCompressedDiffInfo diffInfo;
    if (getSingleCompressedDiffInfo(&diffInfo, &diffData.base, 0))
    {
        PatchState state = { 0 };
        sspatch_listener_t listener;
        memset(&listener, 0, sizeof(listener));

        listener.import = &state;
        listener.onDiffInfo = on_diff_info;
        listener.onPatchFinish = on_patch_finish;

        result = patch_single_stream(&listener, &outData.base, &oldData.base, &diffData.base, 0, NULL, 0);
    }

cleanup:
    hpatch_TFileStreamOutput_close(&outData);
    hpatch_TFileStreamInput_close(&diffData);
    hpatch_TFileStreamInput_close(&oldData);
    return result;
}

int hpatch_patch_file(const char* old_path,
                      const char* patch_path,
                      const char* out_path)
{
    int in_place = (old_path && out_path && strcmp(old_path, out_path) == 0);

    if (!in_place)
        return execute_patch(old_path, patch_path, out_path) ? 0 : -1;

    char tmp_path[4096];
    snprintf(tmp_path, sizeof(tmp_path), "%s.tmp", out_path);

    if (execute_patch(old_path, patch_path, tmp_path))
    {
#ifdef _WIN32
        remove(out_path);
#endif
        if (rename(tmp_path, out_path) == 0)
            return 0;
    }

    remove(tmp_path);
    return -1;
}