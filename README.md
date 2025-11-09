# Scrollr Backend

## Logging
Logs are stored in a .log file in the project's root directory as well as printed to the terminal. The terminal will have the most up to date information, as the current state of the logger is to wait until the allotted internal buffer reaches a size of 8,192 bytes at which point it will flush the internal buffer to the file. This is meant to reduce the frequency of what would otherwise be available threads from getting held up by less pertinent I/O operations.