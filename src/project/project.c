/*
 * Project management: the project list, the selection and the
 * paths of the project settings files.
 *
 * Format of im_config/project_config.json:
 *
 *     {
 *         "projects": [
 *             {
 *                 "project_id": 1,
 *                 "project_name": "Arduino logger",
 *                 "project_protocol": "uart",
 *                 "project_active": true
 *             }
 *         ],
 *         "selected_project_id": 1
 *     }
 *
 * "selected_project_id" is absent when no project is selected.
 *
 * Every public function loads the file, works on the document
 * in memory and, when it changes something, writes the whole
 * document back.
 */

#include "project/project.h"

#include "storage/config_dir.h"
#include "storage/json_file.h"

#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/*
 * Keys used in project_config.json.
 */
#define KEY_SELECTED_PROJECT_ID "selected_project_id"
#define KEY_PROJECTS "projects"
#define KEY_PROJECT_ID "project_id"
#define KEY_PROJECT_NAME "project_name"
#define KEY_PROJECT_PROTOCOL "project_protocol"
#define KEY_PROJECT_ACTIVE "project_active"

/*
 * Loads the project configuration file.
 *
 * On success *root holds a new reference to the document and
 * *projects points to its "projects" array (borrowed from *root).
 * The caller releases *root with json_decref().
 *
 * A missing file is not an error: an empty document is returned.
 */
static int load_document(
    char *path,
    size_t path_size,
    json_t **root,
    json_t **projects
)
{
    json_t *document = NULL;

    if (config_build_path(
            path,
            path_size,
            PROJECT_CONFIG_FILE_NAME) != 0) {
        return -1;
    }

    if (json_file_read(path, &document) != 0) {

        if (errno != ENOENT) {
            return -1;
        }

        document = json_pack("{s:[]}", KEY_PROJECTS);

        if (document == NULL) {
            errno = ENOMEM;
            return -1;
        }
    }

    json_t *array = json_object_get(document, KEY_PROJECTS);

    if (!json_is_array(array)) {
        fprintf(
            stderr,
            "%s: missing '%s' array\n",
            path,
            KEY_PROJECTS
        );
        json_decref(document);
        errno = EINVAL;
        return -1;
    }

    *root = document;
    *projects = array;

    return 0;
}

/*
 * Converts one entry of the "projects" array.
 */
static int parse_project(
    const char *path,
    size_t index,
    const json_t *entry,
    project_info_t *project
)
{
    const json_t *id = json_object_get(entry, KEY_PROJECT_ID);
    const json_t *name = json_object_get(entry, KEY_PROJECT_NAME);
    const json_t *protocol = json_object_get(
        entry,
        KEY_PROJECT_PROTOCOL
    );
    const json_t *active = json_object_get(
        entry,
        KEY_PROJECT_ACTIVE
    );

    if (!json_is_integer(id) ||
        json_integer_value(id) < 1 ||
        json_integer_value(id) > INT_MAX ||
        !json_is_string(name) ||
        !json_is_string(protocol) ||
        !json_is_boolean(active)) {

        fprintf(
            stderr,
            "%s: invalid project at index %zu\n",
            path,
            index
        );
        errno = EINVAL;
        return -1;
    }

    memset(project, 0, sizeof(*project));

    project->id = (int)json_integer_value(id);
    project->active = json_is_true(active);

    snprintf(
        project->name,
        sizeof(project->name),
        "%s",
        json_string_value(name)
    );

    snprintf(
        project->protocol,
        sizeof(project->protocol),
        "%s",
        json_string_value(protocol)
    );

    return 0;
}

/*
 * Looks for the project with the given id.
 *
 * On success *entry points to the JSON object of the project
 * (borrowed from the array) and *project holds its data.
 *
 * errno is ENOENT when there is no such project.
 */
static int find_project(
    const char *path,
    json_t *projects,
    int id,
    json_t **entry,
    project_info_t *project
)
{
    for (size_t i = 0; i < json_array_size(projects); i++) {
        json_t *current = json_array_get(projects, i);

        if (parse_project(path, i, current, project) != 0) {
            return -1;
        }

        if (project->id == id) {
            *entry = current;
            return 0;
        }
    }

    errno = ENOENT;
    return -1;
}

/*
 * Releases the document and keeps errno of the failed operation.
 */
static int fail(json_t *root, int error)
{
    json_decref(root);
    errno = error;

    return -1;
}

int project_initialize_storage(void)
{
    char path[PATH_MAX];

    /*
     * Creates im_config and im_config/projects.
     */
    if (project_create_settings_directory() != 0 ||
        config_build_path(
            path,
            sizeof(path),
            PROJECT_CONFIG_FILE_NAME) != 0) {
        return -1;
    }

    if (access(path, F_OK) == 0) {
        return 0;
    }

    json_t *root = json_pack("{s:[]}", KEY_PROJECTS);

    if (root == NULL) {
        errno = ENOMEM;
        return -1;
    }

    if (json_file_write(path, root) != 0) {
        return fail(root, errno);
    }

    json_decref(root);

    return 0;
}

int project_add(
    const char *name,
    const char *protocol,
    project_info_t *project
)
{
    char path[PATH_MAX];
    json_t *root = NULL;
    json_t *projects = NULL;

    if (name == NULL ||
        protocol == NULL ||
        *name == '\0' ||
        *protocol == '\0' ||
        strlen(name) >= PROJECT_NAME_MAX ||
        strlen(protocol) >= PROJECT_PROTOCOL_MAX) {

        errno = EINVAL;
        return -1;
    }

    /*
     * Make sure im_config exists, then load the current list.
     */
    if (config_create_directory(NULL) != 0 ||
        load_document(path, sizeof(path), &root, &projects) != 0) {
        return -1;
    }

    /*
     * Find the highest id that is already in use.
     */
    int highest_id = 0;

    for (size_t i = 0; i < json_array_size(projects); i++) {
        project_info_t existing;

        if (parse_project(
                path,
                i,
                json_array_get(projects, i),
                &existing) != 0) {
            return fail(root, EINVAL);
        }

        if (existing.id > highest_id) {
            highest_id = existing.id;
        }
    }

    if (highest_id == INT_MAX) {
        return fail(root, EOVERFLOW);
    }

    int id = highest_id + 1;

    /*
     * json_pack() returns NULL when a string is not valid UTF-8.
     */
    json_t *entry = json_pack(
        "{s:i, s:s, s:s, s:b}",
        KEY_PROJECT_ID, id,
        KEY_PROJECT_NAME, name,
        KEY_PROJECT_PROTOCOL, protocol,
        KEY_PROJECT_ACTIVE, 1
    );

    if (entry == NULL) {
        return fail(root, EINVAL);
    }

    /*
     * Append the project and write the whole document back.
     */
    if (json_array_append_new(projects, entry) != 0) {
        return fail(root, ENOMEM);
    }

    if (json_file_write(path, root) != 0) {
        return fail(root, errno);
    }

    json_decref(root);

    /*
     * Report the stored data back to the caller.
     */
    if (project != NULL) {
        memset(project, 0, sizeof(*project));

        project->id = id;
        project->active = true;

        snprintf(
            project->name,
            sizeof(project->name),
            "%s",
            name
        );

        snprintf(
            project->protocol,
            sizeof(project->protocol),
            "%s",
            protocol
        );
    }

    return 0;
}

int project_get_all(
    project_info_t *projects,
    size_t max_projects,
    size_t *project_count
)
{
    char path[PATH_MAX];
    json_t *root = NULL;
    json_t *array = NULL;

    if (projects == NULL ||
        project_count == NULL ||
        max_projects == 0) {

        errno = EINVAL;
        return -1;
    }

    *project_count = 0;

    if (load_document(path, sizeof(path), &root, &array) != 0) {
        return -1;
    }

    /*
     * The caller provides the buffer, so a list that does not
     * fit is refused instead of being cut.
     */
    size_t count = json_array_size(array);

    if (count > max_projects) {
        return fail(root, ENOSPC);
    }

    for (size_t i = 0; i < count; i++) {

        if (parse_project(
                path,
                i,
                json_array_get(array, i),
                &projects[i]) != 0) {
            return fail(root, EINVAL);
        }
    }

    json_decref(root);

    *project_count = count;

    return 0;
}

int project_get_by_id(
    int id,
    project_info_t *project
)
{
    char path[PATH_MAX];
    json_t *root = NULL;
    json_t *array = NULL;
    json_t *entry = NULL;

    if (project == NULL) {
        errno = EINVAL;
        return -1;
    }

    if (load_document(path, sizeof(path), &root, &array) != 0) {
        return -1;
    }

    if (find_project(path, array, id, &entry, project) != 0) {
        return fail(root, errno);
    }

    json_decref(root);

    return 0;
}

int project_set_active(
    int id,
    bool active
)
{
    char path[PATH_MAX];
    json_t *root = NULL;
    json_t *array = NULL;
    json_t *entry = NULL;
    project_info_t project;

    if (load_document(path, sizeof(path), &root, &array) != 0) {
        return -1;
    }

    /*
     * Find the JSON object of the project. Its flag is changed
     * below and the whole document is written back.
     */
    if (find_project(path, array, id, &entry, &project) != 0) {
        return fail(root, errno);
    }

    if (json_object_set_new(
            entry,
            KEY_PROJECT_ACTIVE,
            json_boolean(active)) != 0) {
        return fail(root, ENOMEM);
    }

    /*
     * An inactive project cannot stay selected.
     */
    const json_t *selected = json_object_get(
        root,
        KEY_SELECTED_PROJECT_ID
    );

    if (!active &&
        json_is_integer(selected) &&
        json_integer_value(selected) == id) {

        json_object_del(root, KEY_SELECTED_PROJECT_ID);
    }

    if (json_file_write(path, root) != 0) {
        return fail(root, errno);
    }

    json_decref(root);

    return 0;
}

int project_select(int id)
{
    char path[PATH_MAX];
    json_t *root = NULL;
    json_t *array = NULL;
    json_t *entry = NULL;
    project_info_t project;

    if (load_document(path, sizeof(path), &root, &array) != 0) {
        return -1;
    }

    /*
     * The project has to exist and be active.
     */
    if (find_project(path, array, id, &entry, &project) != 0) {
        return fail(root, errno);
    }

    if (!project.active) {
        return fail(root, EINVAL);
    }

    if (json_object_set_new(
            root,
            KEY_SELECTED_PROJECT_ID,
            json_integer(id)) != 0) {
        return fail(root, ENOMEM);
    }

    if (json_file_write(path, root) != 0) {
        return fail(root, errno);
    }

    json_decref(root);

    return 0;
}

int project_get_selected(project_info_t *project)
{
    char path[PATH_MAX];
    json_t *root = NULL;
    json_t *array = NULL;
    json_t *entry = NULL;

    if (project == NULL) {
        errno = EINVAL;
        return -1;
    }

    if (load_document(path, sizeof(path), &root, &array) != 0) {
        return -1;
    }

    /*
     * A missing or malformed selection means that no project
     * is selected.
     */
    const json_t *selected = json_object_get(
        root,
        KEY_SELECTED_PROJECT_ID
    );

    if (!json_is_integer(selected) ||
        json_integer_value(selected) < 1 ||
        json_integer_value(selected) > INT_MAX) {
        return fail(root, ENOENT);
    }

    /*
     * The selection may point to a project that is no longer
     * in the file. That is reported as ENOENT as well.
     */
    if (find_project(
            path,
            array,
            (int)json_integer_value(selected),
            &entry,
            project) != 0) {
        return fail(root, errno);
    }

    json_decref(root);

    return 0;
}

int project_get_settings_path(
    int id,
    char *buffer,
    size_t size
)
{
    /*
     * The settings file is named after the project id:
     * projects/<id>.json
     */
    char relative_path[64];

    snprintf(
        relative_path,
        sizeof(relative_path),
        "%s/%d.json",
        PROJECT_SETTINGS_DIRECTORY_NAME,
        id
    );

    return config_build_path(buffer, size, relative_path);
}

bool project_settings_exist(int id)
{
    char path[PATH_MAX];

    if (project_get_settings_path(id, path, sizeof(path)) != 0) {
        return false;
    }

    return access(path, F_OK) == 0;
}

int project_create_settings_directory(void)
{
    return config_create_directory(PROJECT_SETTINGS_DIRECTORY_NAME);
}
