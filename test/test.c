#include <sys/attr.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>

// Define the structure for attributes we need
struct attrlist attrlist;
struct {
    uint32_t length;                // Length of this structure
    attrreference_t name_info;      // Reference to the file name
    off_t total_size;               // Logical file size
} attrbuf;

// Function to retrieve metadata
void get_directory_metadata(const char *directory_path) {
    // Initialize the attribute list
    memset(&attrlist, 0, sizeof(attrlist));
    attrlist.bitmapcount = ATTR_BIT_MAP_COUNT;
    attrlist.commonattr = ATTR_CMN_NAME | ATTR_BULK_REQUIRED;         // For file name
    attrlist.fileattr = ATTR_FILE_TOTALSIZE;     // For logical size

    // Open the directory
    int dir_fd = open(directory_path, O_RDONLY);
    if (dir_fd < 0) {
        perror("Failed to open directory");
        return;
    }

    // Read attributes in bulk
    ssize_t count;
    while ((count = getattrlistbulk(dir_fd, &attrlist, &attrbuf, sizeof(attrbuf), 0)) > 0) {
        // Parse results for each file
        for (ssize_t i = 0; i < count; ++i) {
            char *name = (char *)(&attrbuf) + attrbuf.name_info.attr_dataoffset;
            printf("File: %s, Size: %lld bytes\n", name, attrbuf.total_size);
        }
    }

    if (count < 0) {
        perror("getattrlistbulk failed");
    }

    close(dir_fd);
}

int main() {
    const char *directory_path = "/Users/benjaminxu/Desktop";
    get_directory_metadata(directory_path);
    return 0;
}
