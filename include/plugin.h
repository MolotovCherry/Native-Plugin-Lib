#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct PluginGuard PluginGuard;

/**
 * A plugin string.
 *
 * # Safety
 * This points to a valid utf-8 string
 * This does not contain a null terminator
 * This is only valid for reads up to `len`
 */
typedef struct PluginStr {
  const char *ptr;
  uintptr_t len;
} PluginStr;

typedef struct Version {
  uint16_t major;
  uint16_t minor;
  uint16_t patch;
} Version;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Get a plugin's data
 *
 * Takes in a path to the dll, encoded as UTF16
 * Returns null pointer if it failed, non-null if it succeeded.
 * If it failed, either the plugin didn't declare it, it's not a plugin made with Rust Native template,
 * or the file does not exist.
 *
 * # Safety
 * `dll` must be a null terminated utf-16 string
 */
const struct PluginGuard *get_plugin_data(const uint16_t *dll);

/**
 * Get the plugin name
 *
 * # Safety
 * Must be pointer to a valid instance of PluginGuard
 */
struct PluginStr name(const struct PluginGuard *plugin);

/**
 * Get the plugin author
 *
 * # Safety
 * Must be pointer to a valid instance of PluginGuard
 */
struct PluginStr author(const struct PluginGuard *plugin);

/**
 * Get the plugin description
 *
 * # Safety
 * Must be pointer to a valid instance of PluginGuard
 */
struct PluginStr description(const struct PluginGuard *plugin);

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
