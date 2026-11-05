// The C boundary between the Rust host and a CDM's C++ interface.
//
// It names no interface version: each version's C++ file (cdm_11.cc) fills a WvCdmCalls table
// for its own `ContentDecryptionModule_N` and forwards its `Host_N`'s calls to a WvHostCalls
// table. The Rust side (src/boundary.rs) declares the same structures, field for field, in the
// same order; a change here is made there too, and a test compares their sizes on both sides.

#ifndef WIDEVINE_SHIM_BOUNDARY_H_
#define WIDEVINE_SHIM_BOUNDARY_H_

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// One key's status, laid out as the header's `cdm::KeyInformation`.
typedef struct WvKeyInfo {
  const uint8_t* key_id;
  uint32_t key_id_size;
  uint32_t status;
  uint32_t system_code;
} WvKeyInfo;

// One subsample run, laid out as the header's `cdm::SubsampleEntry`.
typedef struct WvSubsample {
  uint32_t clear_bytes;
  uint32_t cipher_bytes;
} WvSubsample;

// One sample to decrypt; the values of the header's `InputBuffer_2`.
typedef struct WvDecryptInput {
  const uint8_t* data;
  uint32_t data_size;
  uint32_t scheme;
  const uint8_t* key_id;
  uint32_t key_id_size;
  const uint8_t* iv;
  uint32_t iv_size;
  const WvSubsample* subsamples;
  uint32_t subsample_count;
  uint32_t crypt_byte_block;
  uint32_t skip_byte_block;
  int64_t timestamp;
} WvDecryptInput;

// What a decrypt left: the CDM's buffer, if it set one, and its bytes.
typedef struct WvDecryptOutput {
  const uint8_t* data;
  uint32_t size;
  void* buffer;
} WvDecryptOutput;

// The host's side: every call the CDM makes on its host, with the host's own pointer first.
typedef struct WvHostCalls {
  void (*set_timer)(void* host, int64_t delay_ms, void* context);
  double (*wall_time)(void* host);
  void (*on_initialized)(void* host, bool success);
  void (*on_resolve_key_status)(void* host, uint32_t promise_id, uint32_t key_status);
  void (*on_resolve_new_session)(void* host, uint32_t promise_id, const char* session_id,
                                 uint32_t session_id_size);
  void (*on_resolve)(void* host, uint32_t promise_id);
  void (*on_reject)(void* host, uint32_t promise_id, uint32_t exception, uint32_t system_code,
                    const char* message, uint32_t message_size);
  void (*on_session_message)(void* host, const char* session_id, uint32_t session_id_size,
                             uint32_t message_type, const char* message, uint32_t message_size);
  void (*on_keys_change)(void* host, const char* session_id, uint32_t session_id_size,
                         bool has_additional_usable_key, const WvKeyInfo* keys,
                         uint32_t key_count);
  void (*on_expiration_change)(void* host, const char* session_id, uint32_t session_id_size,
                               double new_expiry_time);
  void (*on_session_closed)(void* host, const char* session_id, uint32_t session_id_size);
  void (*request_storage_id)(void* host, uint32_t version);
  // A call the host does not answer; `request` is one of the WV_UNANSWERED_ values.
  void (*unanswered)(void* host, uint32_t request);
} WvHostCalls;

#define WV_UNANSWERED_QUERY_OUTPUT_PROTECTION 0u
#define WV_UNANSWERED_ENABLE_OUTPUT_PROTECTION 1u
#define WV_UNANSWERED_PLATFORM_CHALLENGE 2u
#define WV_UNANSWERED_FILE_IO 3u
#define WV_UNANSWERED_DEFERRED_INITIALIZATION 4u
#define WV_UNANSWERED_METRICS 5u

// The CDM's side: every call the host makes on one interface version.
typedef struct WvCdmCalls {
  // sizeof(WvCdmCalls), so that the Rust side can check that it agrees.
  uint32_t size;
  uint32_t interface_version;
  // Calls `entry` (the library's `CreateCdmInstance`) for this version; null when it fails.
  void* (*create)(void* entry, const char* key_system, uint32_t key_system_size, void* host,
                  const WvHostCalls* host_calls);
  void (*initialize)(void* cdm, bool allow_distinctive_identifier, bool allow_persistent_state,
                     bool use_hw_secure_codecs);
  void (*create_session)(void* cdm, uint32_t promise_id, uint32_t session_type,
                         uint32_t init_data_type, const uint8_t* init_data,
                         uint32_t init_data_size);
  void (*update_session)(void* cdm, uint32_t promise_id, const char* session_id,
                         uint32_t session_id_size, const uint8_t* response,
                         uint32_t response_size);
  void (*close_session)(void* cdm, uint32_t promise_id, const char* session_id,
                        uint32_t session_id_size);
  void (*timer_expired)(void* cdm, void* context);
  uint32_t (*decrypt)(void* cdm, const WvDecryptInput* input, WvDecryptOutput* output);
  void (*release_buffer)(void* buffer);
  void (*storage_id)(void* cdm, uint32_t version, const uint8_t* storage_id,
                     uint32_t storage_id_size);
  void (*destroy)(void* cdm);
} WvCdmCalls;

// The table of interface 11.
const WvCdmCalls* widevine_cdm_calls_11(void);

// The size of each structure above as the C++ compiler lays it out, so that the Rust side can
// check that its declarations agree (shim/boundary.cc).
typedef struct WvBoundarySizes {
  uint32_t key_info;
  uint32_t subsample;
  uint32_t decrypt_input;
  uint32_t decrypt_output;
  uint32_t host_calls;
  uint32_t cdm_calls;
} WvBoundarySizes;

WvBoundarySizes widevine_boundary_sizes(void);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // WIDEVINE_SHIM_BOUNDARY_H_
