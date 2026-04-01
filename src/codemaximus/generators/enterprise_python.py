import random

from codemaximus.generators.base import SlopGenerator, GeneratedFile
from codemaximus import naming, comments
from codemaximus.native import line_count


class EnterprisePythonGenerator(SlopGenerator):
    name = "enterprise_python"
    language = "python"
    extension = ".py"

    def generate(self, sanity: float, file_index: int) -> GeneratedFile:
        parts: list[str] = []
        cname = naming.class_name(sanity)
        module_name = cname.lower()

        parts.append(f'"""\n{comments.docstring(sanity, cname)}\n\n')
        parts.append(f"This module provides the {cname} implementation\n")
        parts.append("for enterprise-grade workflow orchestration.\n")
        parts.append('"""\n\n')

        stdlib_imports = [
            "from abc import ABC, abstractmethod",
            "from typing import Any, Optional, Union, Protocol, TypeVar, Generic",
            "from dataclasses import dataclass, field",
            "from functools import wraps, lru_cache",
            "from enum import Enum, auto",
            "import logging",
            "import sys",
            "import os",
            "from collections import defaultdict",
            "from contextlib import contextmanager",
        ]
        for imp in random.sample(stdlib_imports, random.randint(4, len(stdlib_imports))):
            parts.append(f"{imp}\n")
        parts.append("\n")

        parts.append("T = TypeVar('T')\n")
        parts.append("U = TypeVar('U')\n")
        for _ in range(random.randint(2, 5)):
            alias_name = naming.class_name(sanity) + "Type"
            parts.append(f"{alias_name} = Union[dict[str, Any], list[Any], None]\n")
        parts.append("\n")
        parts.append("logger = logging.getLogger(__name__)\n\n")

        meta_name = naming.class_name(sanity) + "Meta"
        parts.append(f"\nclass {meta_name}(type):\n")
        parts.append(f'    """{comments.docstring(sanity, meta_name)}"""\n\n')
        parts.append("    _instances: dict[type, Any] = {}\n\n")
        parts.append("    def __call__(cls, *args: Any, **kwargs: Any) -> Any:\n")
        parts.append("        if cls not in cls._instances:\n")
        parts.append("            cls._instances[cls] = super().__call__(*args, **kwargs)\n")
        parts.append("        return cls._instances[cls]\n\n")

        abc_name = "Abstract" + naming.class_name(sanity)
        parts.append(f"\nclass {abc_name}(ABC):\n")
        parts.append(f'    """{comments.docstring(sanity, abc_name)}"""\n\n')
        num_abstract = random.randint(3, 7)
        for _ in range(num_abstract):
            mname = naming.method_name(sanity)
            params = ", ".join(
                f"{naming.var_name(sanity)}: Any"
                for _ in range(random.randint(1, 4))
            )
            parts.append("    @abstractmethod\n")
            parts.append(f"    def {mname}(self, {params}) -> Any:\n")
            parts.append(f"        # {comments.comment(sanity)}\n")
            parts.append("        ...\n\n")

        enum_name = naming.class_name(sanity) + "Status"
        parts.append(f"\nclass {enum_name}(Enum):\n")
        parts.append(f'    """{comments.docstring(sanity, enum_name)}"""\n\n')
        enum_values = [
            "PENDING",
            "ACTIVE",
            "PROCESSING",
            "VALIDATING",
            "TRANSFORMING",
            "ORCHESTRATING",
            "DELEGATING",
            "RESOLVING",
            "FINALIZING",
            "COMPLETED",
            "FAILED",
            "RETRYING",
            "CANCELLED",
            "DEPRECATED",
            "UNKNOWN",
            "ASCENDING",
            "TRANSCENDING",
            "VIBING",
            "EXISTING",
        ]
        for val in random.sample(enum_values, random.randint(6, min(15, len(enum_values)))):
            parts.append(f"    {val} = auto()\n")
        parts.append("\n")

        parts.append(f"\nclass {cname}({abc_name}, metaclass={meta_name}):\n")
        parts.append(f'    """\n    {comments.docstring(sanity, cname)}\n\n')
        for line in comments.block_comment(sanity, "   "):
            parts.append(f"    {line}\n")
        parts.append('    """\n\n')

        num_params = random.randint(8, 15)
        params_list = [f"{naming.var_name(sanity)}: Any = None" for _ in range(num_params)]
        parts.append("    def __init__(\n")
        parts.append("        self,\n")
        for p in params_list:
            parts.append(f"        {p},\n")
        parts.append("    ) -> None:\n")
        parts.append(f'        """{comments.docstring(sanity, "__init__")}"""\n')
        for p in params_list:
            pname = p.split(":")[0]
            parts.append(f"        self._{pname} = {pname}\n")
        parts.append("        self._initialized = True\n")
        parts.append(f"        self._state = {enum_name}.PENDING\n")
        parts.append(f"        logger.info(f'Initialized {cname}')\n\n")

        for p in params_list[:5]:
            pname = p.split(":")[0]
            parts.append("    @property\n")
            parts.append(f"    def {pname}(self) -> Any:\n")
            parts.append(f"        # {comments.comment(sanity)}\n")
            parts.append(f"        return self._{pname}\n\n")
            parts.append(f"    @{pname}.setter\n")
            parts.append(f"    def {pname}(self, value: Any) -> None:\n")
            parts.append(f"        self._{pname} = value\n\n")

        for _ in range(num_abstract):
            mname = naming.method_name(sanity)
            params = ", ".join(
                f"{naming.var_name(sanity)}: Any"
                for _ in range(random.randint(1, 3))
            )
            parts.append(f"    def {mname}(self, {params}) -> Any:\n")
            parts.append(f'        """{comments.docstring(sanity, mname)}"""\n')
            num_body = random.randint(3, 8)
            for _ in range(num_body):
                vname = naming.var_name(sanity)
                parts.append(f"        {vname} = None  # {comments.comment(sanity)}\n")
            parts.append("        return None\n\n")

        parts.append("    @classmethod\n")
        parts.append(f"    def create(cls, **kwargs: Any) -> '{cname}':\n")
        parts.append(f'        """{comments.docstring(sanity, "create")}"""\n')
        parts.append("        return cls(**kwargs)\n\n")

        parts.append(f"    def __enter__(self) -> '{cname}':\n")
        parts.append(f"        self._state = {enum_name}.ACTIVE\n")
        parts.append("        return self\n\n")
        parts.append("    def __exit__(self, *args: Any) -> None:\n")
        parts.append(f"        self._state = {enum_name}.COMPLETED\n\n")

        parts.append("    def __repr__(self) -> str:\n")
        parts.append(f"        return f'{cname}(state={{self._state}})'\n")

        content = "".join(parts)
        filename = f"python/{module_name}_{file_index}{self.extension}"
        return GeneratedFile(
            filename=filename, content=content, line_count=line_count(content)
        )
