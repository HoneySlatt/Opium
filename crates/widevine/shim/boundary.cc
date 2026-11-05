// The sizes of the C boundary's structures as the C++ compiler lays them out. They name no
// interface version; the Rust side compares them with its own declarations in a test.

#include "boundary.h"

extern "C" WvBoundarySizes widevine_boundary_sizes(void) {
  WvBoundarySizes sizes = {};
  sizes.key_info = sizeof(WvKeyInfo);
  sizes.subsample = sizeof(WvSubsample);
  sizes.decrypt_input = sizeof(WvDecryptInput);
  sizes.decrypt_output = sizeof(WvDecryptOutput);
  sizes.host_calls = sizeof(WvHostCalls);
  sizes.cdm_calls = sizeof(WvCdmCalls);
  return sizes;
}
