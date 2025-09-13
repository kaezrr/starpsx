#!/usr/bin/env python3
import subprocess
import glob
import sys
import os


def run_tests(bios_path):
    # Find all exe test files in ./stuff/*_tests/ directories
    test_patterns = ["./stuff/*_tests/*.exe", "./stuff/*_tests/*.EXE"]
    test_paths = []

    for pattern in test_patterns:
        test_paths.extend(glob.glob(pattern))

    test_paths = sorted(set(test_paths))  # Remove duplicates and sort

    if not test_paths:
        print("No test files found in ./stuff/*_tests/ directories")
        return

    print(f"Found {len(test_paths)} test files")
    print("-" * 50)

    for i, test_path in enumerate(test_paths, 1):
        print(f"[{i}/{len(test_paths)}] Running: {test_path}")

        try:
            # Run the emulator and suppress all output (stdout and stderr)
            result = subprocess.run(
                ["cargo", "run", "--release", "--", bios_path, test_path],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                check=False,
            )

            if result.returncode == 0:
                print(f"✓ Completed: {os.path.basename(test_path)}")
            else:
                print(
                    f"✗ Failed: {os.path.basename(test_path)} (exit code: {result.returncode})"
                )

        except KeyboardInterrupt:
            print(f"\nInterrupted at test: {test_path}")
            break
        except Exception as e:
            print(f"✗ Error running {test_path}: {e}")

        print("-" * 50)

    print("All tests completed!")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python run_tests.py <path/to/bios>")
        sys.exit(1)

    bios_path = sys.argv[1]
    run_tests(bios_path)
