#include "cli/commands.h"

#include "project/project.h"
#include "protocol/protocol.h"

#include <errno.h>
#include <limits.h>
#include <stdio.h>

/*
 * Reports a failed project operation.
 */
static void report_project_error(const char *operation, int id)
{
    if (errno == ENOENT) {
        fprintf(stderr, "Project %d does not exist\n", id);
    } else {
        perror(operation);
    }
}

/*
 * Checks that the data file of the project exists and reports
 * a damaged project otherwise.
 */
static int check_project_data(const project_info_t *project)
{
    char path[PATH_MAX];

    if (project_settings_exist(project->id)) {
        return 0;
    }

    if (project_get_settings_path(
            project->id,
            path,
            sizeof(path)) != 0) {

        snprintf(path, sizeof(path), "(unknown)");
    }

    fprintf(
        stderr,
        "Project %d (%s) is damaged: missing data file %s\n",
        project->id,
        project->name,
        path
    );

    return -1;
}

int command_scan(device_type_t device_type)
{
    printf("Scanning for serial devices...\n\n");

    if (print_serial_devices(device_type) != 0) {
        perror("find_serial_devices");
        return 1;
    }

    return 0;
}

int command_project_list(
    const char *program_name,
    project_filter_t filter
)
{
    project_info_t projects[MAX_PROJECTS];
    project_info_t selected;
    size_t project_count = 0;
    size_t listed = 0;
    int selected_id = 0;

    if (project_get_all(
            projects,
            MAX_PROJECTS,
            &project_count
        ) != 0) {

        perror("project_get_all");
        return 1;
    }

    if (project_get_selected(&selected) == 0) {
        selected_id = selected.id;
    }

    for (size_t i = 0; i < project_count; i++) {

        if ((filter == PROJECT_FILTER_ACTIVE &&
             !projects[i].active) ||
            (filter == PROJECT_FILTER_INACTIVE &&
             projects[i].active)) {
            continue;
        }

        if (listed == 0) {
            printf(
                "  %-5s %-30s %-12s %-10s\n",
                "ID",
                "NAME",
                "PROTOCOL",
                "STATUS"
            );
        }

        /*
         * The selected project is marked with '*'.
         */
        printf(
            "%c %-5d %-30s %-12s %-10s\n",
            projects[i].id == selected_id ? '*' : ' ',
            projects[i].id,
            projects[i].name,
            projects[i].protocol,
            projects[i].active ? "active" : "inactive"
        );

        listed++;
    }

    /*
     * Report listed projects whose data file is missing.
     */
    for (size_t i = 0; i < project_count; i++) {

        if ((filter == PROJECT_FILTER_ACTIVE &&
             !projects[i].active) ||
            (filter == PROJECT_FILTER_INACTIVE &&
             projects[i].active)) {
            continue;
        }

        check_project_data(&projects[i]);
    }

    if (listed == 0 && filter == PROJECT_FILTER_INACTIVE) {
        printf("No inactive projects.\n");
    } else if (listed == 0) {
        printf(
            "No projects. Create one with:\n"
            "  %s project add NAME --protocol PROTOCOL\n",
            program_name
        );
    }

    return 0;
}

int command_project_add(
    const char *program_name,
    const char *name,
    const char *protocol_name
)
{
    project_info_t project;

    const protocol_t *protocol = protocol_find(protocol_name);

    if (protocol == NULL) {
        fprintf(
            stderr,
            "Unknown protocol: '%s' (supported:",
            protocol_name
        );

        for (size_t i = 0; i < protocol_count(); i++) {
            fprintf(stderr, " %s", protocol_get(i)->name);
        }

        fprintf(stderr, ")\n");

        return 1;
    }

    if (project_add(name, protocol->name, &project) != 0) {

        if (errno == EINVAL) {
            fprintf(
                stderr,
                "Invalid project name "
                "(expected 1 to %d characters)\n",
                PROJECT_NAME_MAX - 1
            );
        } else {
            perror("project_add");
        }

        return 1;
    }

    printf(
        "Added project %d: %s (%s)\n",
        project.id,
        project.name,
        project.protocol
    );

    if (protocol->init != NULL && protocol->init(&project) != 0) {
        return 1;
    }

    printf(
        "Select it with: %s project select %d\n",
        program_name,
        project.id
    );

    return 0;
}

int command_project_select(int id)
{
    project_info_t project;

    if (project_get_by_id(id, &project) != 0) {
        report_project_error("project_get_by_id", id);
        return 1;
    }

    if (!project.active) {
        fprintf(
            stderr,
            "Project %d is inactive and cannot be selected\n",
            id
        );
        return 1;
    }

    if (check_project_data(&project) != 0) {
        return 1;
    }

    if (project_select(id) != 0) {
        report_project_error("project_select", id);
        return 1;
    }

    printf(
        "Selected project %d: %s (%s)\n",
        project.id,
        project.name,
        project.protocol
    );

    return 0;
}

int command_project_remove(int id)
{
    project_info_t project;

    if (project_get_by_id(id, &project) != 0) {
        report_project_error("project_get_by_id", id);
        return 1;
    }

    if (!project.active) {
        printf("Project %d is already inactive\n", id);
        return 0;
    }

    if (project_set_active(id, false) != 0) {
        report_project_error("project_set_active", id);
        return 1;
    }

    printf(
        "Removed project %d: %s (marked as inactive)\n",
        project.id,
        project.name
    );

    return 0;
}

int command_run(const char *program_name, bool show_help)
{
    project_info_t project;

    if (project_get_selected(&project) != 0) {

        if (errno == ENOENT) {
            fprintf(
                stderr,
                "No project selected. Select one with:\n"
                "  %s project select ID\n",
                program_name
            );
        } else {
            perror("project_get_selected");
        }

        return 1;
    }

    const protocol_t *protocol = protocol_find(project.protocol);

    if (protocol == NULL) {
        fprintf(
            stderr,
            "Project %d uses an unsupported protocol: '%s'\n",
            project.id,
            project.protocol
        );
        return 1;
    }

    if (show_help) {
        protocol->print_help(program_name, &project);
        return 0;
    }

    if (check_project_data(&project) != 0) {
        return 1;
    }

    return protocol->run(&project) == 0 ? 0 : 1;
}
