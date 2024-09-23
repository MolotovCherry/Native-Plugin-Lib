/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * The plugin data version
 * This is used in C interface. Rust users can ignore it
 */
#define DATA_VERSION 1

typedef struct PluginGuard PluginGuard;

/**
 * A ffi safe rust string.
 *
 * # Safety
 * This points to a valid utf-8 string
 * This does not contain a null terminator
 * This is only valid for reads up to `len`
 */
typedef struct RStr {
  const char *data;
  uintptr_t len;
} RStr;

typedef struct Version {
  uint16_t major;
  uint16_t minor;
  uint16_t patch;
} Version;

/**
 * Plugin details; DATA_VERSION 1
 *
 * If you want to identify your own plugin,
 * export a symbol named PLUGIN_DATA containing
 * this data.
 */
typedef struct Plugin {
  uintptr_t data_ver;
  struct RStr name;
  struct RStr author;
  struct RStr description;
  struct Version version;
} Plugin;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Get a plugin's data
 *
 * Takes in a path to the dll, encoded as UTF16, with length `len`
 * Returns null pointer if it failed, non-null if it succeeded.
 * If it failed, either the plugin didn't declare it, it's not a plugin made with Rust Native template,
 * or the file does not exist.
 *
 * # Safety
 * `len` must be the correct size
 */
const struct PluginGuard *get_plugin_data(const uint16_t *dll,
                                          uintptr_t len);

/**
 * Get the plugin name
 *
 * # Safety
 * Must be pointer to a valid instance of PluginGuard
 */
struct RStr name(const struct PluginGuard *plugin);

/**
 * Get the plugin author
 *
 * # Safety
 * Must be pointer to a valid instance of PluginGuard
 */
struct RStr author(const struct PluginGuard *plugin);

/**
 * Get the plugin description
 *
 * # Safety
 * Must be pointer to a valid instance of PluginGuard
 */
struct RStr description(const struct PluginGuard *plugin);

/**
 * Get the plugin version
 *
 * # Safety
 * Must be pointer to a valid instance of PluginGuard
 */
const struct Version *version(const struct PluginGuard *plugin);

/**
 * Free the memory used by PluginGuard
 *
 * # Safety
 * Must be pointer to a valid instance of PluginGuard
 */
void free_plugin(const struct PluginGuard *plugin);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
