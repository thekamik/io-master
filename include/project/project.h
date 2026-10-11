/*
 * Project management.
 *
 * A project has an id, a name, a protocol and an active flag.
 * All projects are listed in im_config/project_config.json.
 * Each project also has its own settings file,
 * im_config/projects/<id>.json, whose content is defined
 * by the protocol of the project.
 */

#ifndef PROJECT_H
#define PROJECT_H

#include <stdbool.h>
#include <stddef.h>

#define PROJECT_CONFIG_FILE_NAME "project_config.json"
#define PROJECT_SETTINGS_DIRECTORY_NAME "projects"

#define PROJECT_NAME_MAX 128
#define PROJECT_PROTOCOL_MAX 32
#define MAX_PROJECTS 256

typedef struct {
    int id;
    char name[PROJECT_NAME_MAX];
    char protocol[PROJECT_PROTOCOL_MAX];
    bool active;
} project_info_t;

/*
 * Prepares the configuration storage. Meant to be called once
 * at program start.
 *
 * Creates whatever is missing:
 *   im_config/
 *   im_config/project_config.json   (with an empty project list)
 *   im_config/projects/
 *
 * Existing directories and files are left untouched.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int project_initialize_storage(void);

/*
 * Adds a project to im_config/project_config.json.
 *
 * The directory and the file are created when they do not exist.
 * The project receives the next free id (highest existing id + 1)
 * and starts as active.
 *
 * project may be NULL. When it is not NULL, it receives the data
 * of the added project, including the assigned id.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; EINVAL when name or protocol is
 *        empty or too long)
 */
int project_add(
    const char *name,
    const char *protocol,
    project_info_t *project
);

/*
 * Reads all projects, both active and inactive.
 *
 * A missing file means that there are no projects yet:
 * the function succeeds and sets *project_count to 0.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOSPC when the file contains
 *        more than max_projects projects)
 */
int project_get_all(
    project_info_t *projects,
    size_t max_projects,
    size_t *project_count
);

/*
 * Reads the project with the given id.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOENT when there is no such project)
 */
int project_get_by_id(
    int id,
    project_info_t *project
);

/*
 * Marks the project with the given id as active or inactive.
 *
 * Projects are never deleted from the file. Removing a project
 * means marking it as inactive. When the selected project
 * becomes inactive, the selection is cleared.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOENT when there is no such project)
 */
int project_set_active(
    int id,
    bool active
);

/*
 * Selects the project used by later invocations of the program.
 *
 * The selection is stored in im_config/project_config.json.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOENT when there is no such
 *        project, EINVAL when the project is inactive)
 */
int project_select(int id);

/*
 * Reads the currently selected project.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set; ENOENT when no project is selected)
 */
int project_get_selected(project_info_t *project);

/*
 * Builds the path of the settings file of a project:
 * im_config/projects/<id>.json
 *
 * The content of that file is defined by the protocol
 * of the project.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int project_get_settings_path(
    int id,
    char *buffer,
    size_t size
);

/*
 * Checks whether the settings file of a project exists.
 *
 * A project that is listed in project_config.json but has no
 * settings file is damaged.
 */
bool project_settings_exist(int id);

/*
 * Creates the directory that holds the project settings files.
 *
 * Returns:
 *   0  - success
 *  -1  - error (errno is set)
 */
int project_create_settings_directory(void);

#endif
