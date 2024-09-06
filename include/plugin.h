#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Plugin details
 */
typedef struct CPlugin CPlugin;

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
const struct CPlugin *get_plugin_data(const uint16_t *dll);

/**
 * Get the plugin name
 *
 * # Safety
 * Must be pointer to a valid instance of CPlugin
 */
const char *name(const struct CPlugin *plugin);

/**
 * Get the plugin author
 *
 * # Safety
 * Must be pointer to a valid instance of CPlugin
 */
const char *author(const struct CPlugin *plugin);

/**
 * Get the plugin description
 *
 * # Safety
 * Must be pointer to a valid instance of CPlugin
 */
const char *description(const struct CPlugin *plugin);

/**
 * Get the plugin version
 *
 * # Safety
 * Must be pointer to a valid instance of CPlugin
 */
const struct Version *version(const struct CPlugin *plugin);

/**
 * Free the memory used by CPlugin
 *
 * # Safety
 * Must be pointer to a valid instance of CPlugin
 */
void free_plugin(const struct CPlugin *plugin);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus
