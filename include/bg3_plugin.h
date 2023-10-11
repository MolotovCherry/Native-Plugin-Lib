#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

struct Version {
  uint16_t major;
  uint16_t minor;
  uint16_t patch;
};

/// Plugin details
struct Plugin {
  uint8_t name[128];
  uint8_t author[50];
  uint8_t description[512];
  Version version;
};

extern "C" {

/// C function to get the path to the DLL, encoded as UTF16
/// Returns null pointer if it failed, non-null if it succeeded.
/// If it failed, either the plugin didn't declare it, or it's not a plugin made with Rust BG3 template
///
/// # Safety
/// `dll` must be a null terminated utf-16 string
const Plugin *get_plugin_data_c(const uint16_t *dll);

/// Free the memory used by Plugin
///
/// # Safety
/// Must be pointer to a valid instance of Plugin
void free_plugin(Plugin *plugin);

} // extern "C"
