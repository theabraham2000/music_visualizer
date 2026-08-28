from pathlib import Path


# ============================================================
# CONFIGURATION
# ============================================================

# Folders to ignore anywhere in the repository.
IGNORE_DIRS = {
    ".git",
    ".github",
    ".idea",
    ".vscode",
    "__pycache__",
    "node_modules",
    "venv",
    ".venv",
    "env",
    ".env",
    "dist",
    "build",
    "coverage",
    "target",
}

# Specific files to ignore.
# You can add filenames or relative paths here.
IGNORE_FILES = {
    "llm_code.md",
    ".gitignore",
    "generate_llm_code.py",
    "README.md",
    "milestones.md",
}

# File extensions that should be treated as code.
# Add/remove extensions according to your project.
CODE_EXTENSIONS = {
    ".py",
    ".js",
    ".jsx",
    ".ts",
    ".tsx",
    ".java",
    ".c",
    ".h",
    ".cpp",
    ".hpp",
    ".cc",
    ".cs",
    ".go",
    ".rs",
    ".rb",
    ".php",
    ".swift",
    ".kt",
    ".kts",
    ".scala",
    ".sh",
    ".bash",
    ".zsh",
    ".fish",
    ".sql",
    ".html",
    ".css",
    ".scss",
    ".sass",
    ".vue",
    ".svelte",
    ".r",
    ".R",
    ".m",
    ".mm",
    ".dart",
    ".lua",
    ".pl",
    ".ex",
    ".exs",
    ".md",
    ".toml",
}

# Output file
OUTPUT_FILE = "llm_code.md"


# ============================================================
# HELPER FUNCTIONS
# ============================================================

def should_ignore_directory(directory: Path) -> bool:
    """Return True if this directory should be ignored."""
    return directory.name in IGNORE_DIRS


def should_ignore_file(file_path: Path, root: Path) -> bool:
    """Return True if this file should be ignored."""

    # Ignore configured filenames
    if file_path.name in IGNORE_FILES:
        return True

    # Ignore configured relative paths
    relative_path = file_path.relative_to(root).as_posix()

    if relative_path in IGNORE_FILES:
        return True

    # Never include the generated output itself
    if file_path.name == OUTPUT_FILE:
        return True

    return False


def get_code_files(root: Path):
    """Recursively find all code files in the repository."""

    code_files = []

    for path in root.rglob("*"):

        # Skip directories
        if path.is_dir():
            continue

        # Skip ignored files
        if should_ignore_file(path, root):
            continue

        # Check whether any parent directory is ignored
        relative_parts = path.relative_to(root).parts

        if any(part in IGNORE_DIRS for part in relative_parts[:-1]):
            continue

        # Only include configured code extensions
        if path.suffix in CODE_EXTENSIONS:
            code_files.append(path)

    return sorted(code_files)


def build_project_structure(root: Path, code_files):
    """Build a simple project tree containing the included code files."""

    lines = ["```text", root.name]

    # Convert paths to relative paths
    relative_files = [
        file_path.relative_to(root)
        for file_path in code_files
    ]

    # Build tree recursively
    tree = {}

    for relative_path in relative_files:
        current = tree

        for part in relative_path.parts:
            current = current.setdefault(part, {})

    def render_tree(node, prefix=""):
        items = sorted(node.items(), key=lambda x: (bool(x[1]), x[0].lower()))

        for index, (name, children) in enumerate(items):
            is_last = index == len(items) - 1

            connector = "└── " if is_last else "├── "
            lines.append(prefix + connector + name)

            if children:
                extension = "    " if is_last else "│   "
                render_tree(children, prefix + extension)

    render_tree(tree)

    lines.append("```")

    return "\n".join(lines)


def read_code_file(file_path: Path) -> str:
    """Read a source file safely."""

    try:
        return file_path.read_text(
            encoding="utf-8",
            errors="replace"
        )
    except Exception as e:
        return f"[Could not read file: {e}]"


def get_language(file_path: Path) -> str:
    """Return a Markdown code fence language."""

    language_map = {
        ".py": "python",
        ".js": "javascript",
        ".jsx": "jsx",
        ".ts": "typescript",
        ".tsx": "tsx",
        ".java": "java",
        ".c": "c",
        ".h": "c",
        ".cpp": "cpp",
        ".hpp": "cpp",
        ".cc": "cpp",
        ".cs": "csharp",
        ".go": "go",
        ".rs": "rust",
        ".rb": "ruby",
        ".php": "php",
        ".swift": "swift",
        ".kt": "kotlin",
        ".kts": "kotlin",
        ".scala": "scala",
        ".sh": "bash",
        ".bash": "bash",
        ".zsh": "zsh",
        ".sql": "sql",
        ".html": "html",
        ".css": "css",
        ".scss": "scss",
        ".sass": "sass",
        ".vue": "vue",
        ".svelte": "svelte",
        ".r": "r",
        ".R": "r",
        ".m": "objective-c",
        ".mm": "objective-c",
        ".dart": "dart",
        ".lua": "lua",
        ".pl": "perl",
        ".ex": "elixir",
        ".exs": "elixir",
    }

    return language_map.get(file_path.suffix, "")


# ============================================================
# MAIN
# ============================================================

def main():

    # The script is expected to live in the repository root.
    root = Path(__file__).resolve().parent

    print(f"Repository: {root}")
    print("Scanning repository...")

    code_files = get_code_files(root)

    print(f"Found {len(code_files)} code files.")

    # --------------------------------------------------------
    # Build Markdown
    # --------------------------------------------------------

    output = []

    output.append("# Codebase Overview")
    output.append("")
    output.append(
        "This document contains the project structure and "
        "the source code of the repository."
    )
    output.append("")

    # --------------------------------------------------------
    # 1. PROJECT STRUCTURE
    # --------------------------------------------------------

    output.append("# 1. Project Structure")
    output.append("")

    structure = build_project_structure(root, code_files)
    output.append(structure)
    output.append("")

    # --------------------------------------------------------
    # 2. CODE FOR EACH FILE
    # --------------------------------------------------------

    output.append("# 2. Source Code")
    output.append("")

    for file_path in code_files:

        relative_path = file_path.relative_to(root).as_posix()
        language = get_language(file_path)
        code = read_code_file(file_path)

        output.append(f"## `{relative_path}`")
        output.append("")

        output.append(f"```{language}")
        output.append(code)
        output.append("```")
        output.append("")

    # --------------------------------------------------------
    # WRITE OUTPUT
    # --------------------------------------------------------

    output_path = root / OUTPUT_FILE

    output_path.write_text(
        "\n".join(output),
        encoding="utf-8"
    )

    print()
    print("Done!")
    print(f"Generated: {output_path}")
    print(f"Included {len(code_files)} code files.")


if __name__ == "__main__":
    main()
