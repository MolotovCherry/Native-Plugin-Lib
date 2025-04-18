/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * The plugin data version
 */
#define DATA_VERSION 1

typedef struct PluginData PluginData;

typedef struct Version {
  uint16_t major;
  uint16_t minor;
  uint16_t patch;
} Version;

/**
 * utf8 null terminated string.
 *
 * # Safety
 * This points to a valid utf-8 string
 * Contains no internal nulls
 * Contains a null terminator
 */
typedef char *RStr;

/**
 * Plugin details; DATA_VERSION 1
 *
 * If you want to identify your own plugin,
 * export a symbol named PLUGIN_DATA containing
 * this data.
 */
typedef struct Plugin {
  /**
   * This MUST be set to `DATA_VERSION`
   */
  uintptr_t data_ver;
  RStr name;
  RStr author;
  RStr description;
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
 * the file does not exist, or you need to update the native plugin lib since the data version is too high
 *
 * # Safety
 * `len` must be the correct. this is the number of u16 elems, _not_ the number of bytes
 */
const struct PluginData *get_plugin_data(const uint16_t *dll,
                                         uintptr_t len);

/**
 * Get the plugin name
 *
 * # Safety
 * Must be pointer to a valid instance of PluginData
 */
const char *get_plugin_name(const struct PluginData *data);

/**
 * Get the plugin author
 *
 * # Safety
 * Must be pointer to a valid instance of PluginData
 */
const char *get_plugin_author(const struct PluginData *data);

/**
 * Get the plugin description
 *
 * # Safety
 * Must be pointer to a valid instance of PluginData
 */
const char *get_plugin_description(const struct PluginData *data);

/**
 * Get the plugin version
 *
 * # Safety
 * Must be pointer to a valid instance of PluginData
 */
const struct Version *get_plugin_version(const struct PluginData *data);

/**
 * Free the memory used by PluginData
 *
 * # Safety
 * Must be pointer to a valid instance of PluginData
 */
void free_plugin_data(const struct PluginData *data);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
