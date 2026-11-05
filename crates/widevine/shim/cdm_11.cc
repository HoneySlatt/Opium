// Interface 11 of Chromium's CDM interface, passed across the C boundary of boundary.h.
//
// This file only translates: the CDM's calls on its host go to the Rust host's WvHostCalls, and
// the host's calls go to the CDM's `ContentDecryptionModule_11`. It decides nothing, keeps no
// state beyond what a call needs, starts no thread and takes no lock. Every function the Rust
// side calls is `noexcept`: an exception from the CDM ends the process rather than unwinding
// into Rust.

#include <cstddef>
#include <new>

#include "boundary.h"
#include "content_decryption_module.h"

namespace {

static_assert(sizeof(WvKeyInfo) == sizeof(cdm::KeyInformation), "key information layout");
static_assert(offsetof(WvKeyInfo, key_id) == offsetof(cdm::KeyInformation, key_id), "key_id");
static_assert(offsetof(WvKeyInfo, key_id_size) == offsetof(cdm::KeyInformation, key_id_size),
              "key_id_size");
static_assert(offsetof(WvKeyInfo, status) == offsetof(cdm::KeyInformation, status), "status");
static_assert(offsetof(WvKeyInfo, system_code) == offsetof(cdm::KeyInformation, system_code),
              "system_code");
static_assert(sizeof(cdm::KeyStatus) == sizeof(uint32_t), "key status width");
static_assert(sizeof(WvSubsample) == sizeof(cdm::SubsampleEntry), "subsample layout");
static_assert(offsetof(WvSubsample, clear_bytes) == offsetof(cdm::SubsampleEntry, clear_bytes),
              "clear_bytes");
static_assert(offsetof(WvSubsample, cipher_bytes) ==
                  offsetof(cdm::SubsampleEntry, cipher_bytes),
              "cipher_bytes");

// A buffer the CDM asks for through `Allocate`, freed by its `Destroy`.
class HostBuffer final : public cdm::Buffer {
 public:
  static HostBuffer* Create(uint32_t capacity) {
    // The header asks for non-null members, so an empty buffer still owns one byte.
    uint8_t* data = new (std::nothrow) uint8_t[capacity == 0 ? 1 : capacity];
    if (data == nullptr) {
      return nullptr;
    }
    HostBuffer* buffer = new (std::nothrow) HostBuffer(data, capacity);
    if (buffer == nullptr) {
      delete[] data;
    }
    return buffer;
  }

  void Destroy() override { delete this; }
  uint32_t Capacity() const override { return capacity_; }
  uint8_t* Data() override { return data_; }
  void SetSize(uint32_t size) override { size_ = size; }
  uint32_t Size() const override { return size_; }

 private:
  HostBuffer(uint8_t* data, uint32_t capacity) : data_(data), capacity_(capacity) {}
  ~HostBuffer() override { delete[] data_; }

  uint8_t* data_;
  uint32_t capacity_;
  uint32_t size_ = 0;
};

// Where the CDM puts a decrypt's cleartext; it lives for one `Decrypt` call.
class DecryptedBlock final : public cdm::DecryptedBlock {
 public:
  DecryptedBlock() = default;
  ~DecryptedBlock() override = default;

  void SetDecryptedBuffer(cdm::Buffer* buffer) override { buffer_ = buffer; }
  cdm::Buffer* DecryptedBuffer() override { return buffer_; }
  void SetTimestamp(int64_t timestamp) override { timestamp_ = timestamp; }
  int64_t Timestamp() const override { return timestamp_; }

 private:
  cdm::Buffer* buffer_ = nullptr;
  int64_t timestamp_ = 0;
};

// The host the CDM sees: each call goes to the Rust host.
class Host final : public cdm::Host_11 {
 public:
  Host(void* rust_host, const WvHostCalls* calls) : rust_host_(rust_host), calls_(calls) {}
  ~Host() override = default;

  cdm::Buffer* Allocate(uint32_t capacity) override { return HostBuffer::Create(capacity); }

  void SetTimer(int64_t delay_ms, void* context) override {
    calls_->set_timer(rust_host_, delay_ms, context);
  }

  cdm::Time GetCurrentWallTime() override { return calls_->wall_time(rust_host_); }

  void OnInitialized(bool success) override { calls_->on_initialized(rust_host_, success); }

  void OnResolveKeyStatusPromise(uint32_t promise_id, cdm::KeyStatus key_status) override {
    calls_->on_resolve_key_status(rust_host_, promise_id, key_status);
  }

  void OnResolveNewSessionPromise(uint32_t promise_id, const char* session_id,
                                  uint32_t session_id_size) override {
    calls_->on_resolve_new_session(rust_host_, promise_id, session_id, session_id_size);
  }

  void OnResolvePromise(uint32_t promise_id) override {
    calls_->on_resolve(rust_host_, promise_id);
  }

  void OnRejectPromise(uint32_t promise_id, cdm::Exception exception, uint32_t system_code,
                       const char* error_message, uint32_t error_message_size) override {
    calls_->on_reject(rust_host_, promise_id, exception, system_code, error_message,
                      error_message_size);
  }

  void OnSessionMessage(const char* session_id, uint32_t session_id_size,
                        cdm::MessageType message_type, const char* message,
                        uint32_t message_size) override {
    calls_->on_session_message(rust_host_, session_id, session_id_size, message_type, message,
                               message_size);
  }

  void OnSessionKeysChange(const char* session_id, uint32_t session_id_size,
                           bool has_additional_usable_key, const cdm::KeyInformation* keys_info,
                           uint32_t keys_info_count) override {
    // The layouts are the same (checked above); the Rust side reads the entries.
    calls_->on_keys_change(rust_host_, session_id, session_id_size, has_additional_usable_key,
                           reinterpret_cast<const WvKeyInfo*>(keys_info), keys_info_count);
  }

  void OnExpirationChange(const char* session_id, uint32_t session_id_size,
                          cdm::Time new_expiry_time) override {
    calls_->on_expiration_change(rust_host_, session_id, session_id_size, new_expiry_time);
  }

  void OnSessionClosed(const char* session_id, uint32_t session_id_size) override {
    calls_->on_session_closed(rust_host_, session_id, session_id_size);
  }

  void SendPlatformChallenge(const char*, uint32_t, const char*, uint32_t) override {
    calls_->unanswered(rust_host_, WV_UNANSWERED_PLATFORM_CHALLENGE);
  }

  void EnableOutputProtection(uint32_t) override {
    calls_->unanswered(rust_host_, WV_UNANSWERED_ENABLE_OUTPUT_PROTECTION);
  }

  void QueryOutputProtectionStatus() override {
    calls_->unanswered(rust_host_, WV_UNANSWERED_QUERY_OUTPUT_PROTECTION);
  }

  void OnDeferredInitializationDone(cdm::StreamType, cdm::Status) override {
    calls_->unanswered(rust_host_, WV_UNANSWERED_DEFERRED_INITIALIZATION);
  }

  // The host has no file to give: the Rust side is told, and the CDM gets none.
  cdm::FileIO* CreateFileIO(cdm::FileIOClient*) override {
    calls_->unanswered(rust_host_, WV_UNANSWERED_FILE_IO);
    return nullptr;
  }

  void RequestStorageId(uint32_t version) override {
    calls_->request_storage_id(rust_host_, version);
  }

  void ReportMetrics(cdm::MetricName, uint64_t) override {
    calls_->unanswered(rust_host_, WV_UNANSWERED_METRICS);
  }

 private:
  void* rust_host_;
  const WvHostCalls* calls_;
};

// One CDM instance and the host it was given.
struct Instance {
  Instance(void* rust_host, const WvHostCalls* calls) : host(rust_host, calls) {}

  Host host;
  cdm::ContentDecryptionModule_11* cdm = nullptr;
};

Instance* AsInstance(void* cdm) { return static_cast<Instance*>(cdm); }

using CreateCdmInstanceFunc = decltype(&CreateCdmInstance);

// The host lookup given to `CreateCdmInstance`: it answers interface 11 only.
void* GetHost(int host_interface_version, void* user_data) {
  return host_interface_version == cdm::Host_11::kVersion ? user_data : nullptr;
}

void* Create(void* entry, const char* key_system, uint32_t key_system_size, void* rust_host,
             const WvHostCalls* host_calls) noexcept {
  Instance* instance = new (std::nothrow) Instance(rust_host, host_calls);
  if (instance == nullptr) {
    return nullptr;
  }
  CreateCdmInstanceFunc create_cdm_instance = reinterpret_cast<CreateCdmInstanceFunc>(entry);
  cdm::Host_11* host = &instance->host;
  void* created = create_cdm_instance(cdm::ContentDecryptionModule_11::kVersion, key_system,
                                      key_system_size, &GetHost, host);
  if (created == nullptr) {
    delete instance;
    return nullptr;
  }
  instance->cdm = static_cast<cdm::ContentDecryptionModule_11*>(created);
  return instance;
}

void Initialize(void* cdm, bool allow_distinctive_identifier, bool allow_persistent_state,
                bool use_hw_secure_codecs) noexcept {
  AsInstance(cdm)->cdm->Initialize(allow_distinctive_identifier, allow_persistent_state,
                                   use_hw_secure_codecs);
}

void CreateSession(void* cdm, uint32_t promise_id, uint32_t session_type,
                   uint32_t init_data_type, const uint8_t* init_data,
                   uint32_t init_data_size) noexcept {
  AsInstance(cdm)->cdm->CreateSessionAndGenerateRequest(
      promise_id, static_cast<cdm::SessionType>(session_type),
      static_cast<cdm::InitDataType>(init_data_type), init_data, init_data_size);
}

void UpdateSession(void* cdm, uint32_t promise_id, const char* session_id,
                   uint32_t session_id_size, const uint8_t* response,
                   uint32_t response_size) noexcept {
  AsInstance(cdm)->cdm->UpdateSession(promise_id, session_id, session_id_size, response,
                                      response_size);
}

void CloseSession(void* cdm, uint32_t promise_id, const char* session_id,
                  uint32_t session_id_size) noexcept {
  AsInstance(cdm)->cdm->CloseSession(promise_id, session_id, session_id_size);
}

void TimerExpired(void* cdm, void* context) noexcept {
  AsInstance(cdm)->cdm->TimerExpired(context);
}

uint32_t Decrypt(void* cdm, const WvDecryptInput* input, WvDecryptOutput* output) noexcept {
  cdm::InputBuffer_2 buffer = {};
  buffer.data = input->data;
  buffer.data_size = input->data_size;
  buffer.encryption_scheme = static_cast<cdm::EncryptionScheme>(input->scheme);
  buffer.key_id = input->key_id;
  buffer.key_id_size = input->key_id_size;
  buffer.iv = input->iv;
  buffer.iv_size = input->iv_size;
  // The layouts are the same (checked above).
  buffer.subsamples = reinterpret_cast<const cdm::SubsampleEntry*>(input->subsamples);
  buffer.num_subsamples = input->subsample_count;
  buffer.pattern.crypt_byte_block = input->crypt_byte_block;
  buffer.pattern.skip_byte_block = input->skip_byte_block;
  buffer.timestamp = input->timestamp;

  DecryptedBlock block;
  cdm::Status status = AsInstance(cdm)->cdm->Decrypt(buffer, &block);
  cdm::Buffer* decrypted = block.DecryptedBuffer();
  output->buffer = decrypted;
  output->data = decrypted == nullptr ? nullptr : decrypted->Data();
  output->size = decrypted == nullptr ? 0 : decrypted->Size();
  return status;
}

void ReleaseBuffer(void* buffer) noexcept { static_cast<cdm::Buffer*>(buffer)->Destroy(); }

void StorageId(void* cdm, uint32_t version, const uint8_t* storage_id,
               uint32_t storage_id_size) noexcept {
  AsInstance(cdm)->cdm->OnStorageId(version, storage_id, storage_id_size);
}

void Destroy(void* cdm) noexcept {
  Instance* instance = AsInstance(cdm);
  instance->cdm->Destroy();
  delete instance;
}

const WvCdmCalls kCalls = {
    sizeof(WvCdmCalls),
    static_cast<uint32_t>(cdm::ContentDecryptionModule_11::kVersion),
    &Create,
    &Initialize,
    &CreateSession,
    &UpdateSession,
    &CloseSession,
    &TimerExpired,
    &Decrypt,
    &ReleaseBuffer,
    &StorageId,
    &Destroy,
};

}  // namespace

extern "C" const WvCdmCalls* widevine_cdm_calls_11(void) { return &kCalls; }
