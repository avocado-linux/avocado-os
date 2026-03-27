#include <gpiod.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <dirent.h>
#include <time.h>

#define DEFAULT_CHIP "/dev/gpiochip0"
#define DEFAULT_LINE 17
#define TOGGLE_INTERVAL_S 1

static void list_chips(void)
{
	DIR *dir;
	struct dirent *entry;

	dir = opendir("/dev");
	if (!dir) {
		perror("opendir /dev");
		return;
	}

	printf("GPIO chips:\n");
	while ((entry = readdir(dir)) != NULL) {
		if (strncmp(entry->d_name, "gpiochip", 8) != 0)
			continue;

		char path[64];
		snprintf(path, sizeof(path), "/dev/%s", entry->d_name);

		struct gpiod_chip *chip = gpiod_chip_open(path);
		if (!chip)
			continue;

		struct gpiod_chip_info *info = gpiod_chip_get_info(chip);
		if (info) {
			printf("  %s [%s] (%zu lines)\n",
			       gpiod_chip_info_get_name(info),
			       gpiod_chip_info_get_label(info),
			       gpiod_chip_info_get_num_lines(info));
			gpiod_chip_info_free(info);
		}

		gpiod_chip_close(chip);
	}
	closedir(dir);
}

int main(int argc, char *argv[])
{
	const char *chip_path = DEFAULT_CHIP;
	unsigned int line_offset = DEFAULT_LINE;

	if (argc > 1)
		chip_path = argv[1];
	if (argc > 2)
		line_offset = (unsigned int)atoi(argv[2]);

	printf("gpio-toggle starting\n");
	fflush(stdout);

	list_chips();
	fflush(stdout);

	printf("Opening %s, line %u\n", chip_path, line_offset);
	fflush(stdout);

	struct gpiod_chip *chip = gpiod_chip_open(chip_path);
	if (!chip) {
		perror("gpiod_chip_open");
		return EXIT_FAILURE;
	}

	struct gpiod_line_settings *settings = gpiod_line_settings_new();
	if (!settings) {
		fprintf(stderr, "Failed to create line settings\n");
		gpiod_chip_close(chip);
		return EXIT_FAILURE;
	}
	gpiod_line_settings_set_direction(settings, GPIOD_LINE_DIRECTION_OUTPUT);
	gpiod_line_settings_set_output_value(settings, GPIOD_LINE_VALUE_INACTIVE);

	struct gpiod_line_config *line_cfg = gpiod_line_config_new();
	if (!line_cfg) {
		fprintf(stderr, "Failed to create line config\n");
		gpiod_line_settings_free(settings);
		gpiod_chip_close(chip);
		return EXIT_FAILURE;
	}
	gpiod_line_config_add_line_settings(line_cfg, &line_offset, 1, settings);

	struct gpiod_request_config *req_cfg = gpiod_request_config_new();
	if (!req_cfg) {
		fprintf(stderr, "Failed to create request config\n");
		gpiod_line_config_free(line_cfg);
		gpiod_line_settings_free(settings);
		gpiod_chip_close(chip);
		return EXIT_FAILURE;
	}
	gpiod_request_config_set_consumer(req_cfg, "gpio-toggle");

	struct gpiod_line_request *request =
		gpiod_chip_request_lines(chip, req_cfg, line_cfg);

	gpiod_request_config_free(req_cfg);
	gpiod_line_config_free(line_cfg);
	gpiod_line_settings_free(settings);

	if (!request) {
		perror("gpiod_chip_request_lines");
		gpiod_chip_close(chip);
		return EXIT_FAILURE;
	}

	printf("Toggling line %u every %ds\n", line_offset, TOGGLE_INTERVAL_S);
	fflush(stdout);

	int value = 0;

	while (1) {
		value = !value;
		enum gpiod_line_value val = value
			? GPIOD_LINE_VALUE_ACTIVE
			: GPIOD_LINE_VALUE_INACTIVE;

		int ret = gpiod_line_request_set_value(request, line_offset, val);
		if (ret < 0) {
			perror("gpiod_line_request_set_value");
			break;
		}

		time_t now = time(NULL);
		printf("[%ld] line %u = %s\n", now, line_offset,
		       value ? "HIGH" : "LOW");
		fflush(stdout);

		sleep(TOGGLE_INTERVAL_S);
	}

	gpiod_line_request_release(request);
	gpiod_chip_close(chip);

	return EXIT_SUCCESS;
}
