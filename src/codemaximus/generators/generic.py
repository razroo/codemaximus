import random

from codemaximus.generators.base import SlopGenerator, GeneratedFile
from codemaximus import naming, comments
from codemaximus.native import line_count, native_generate_generic


class GenericGenerator(SlopGenerator):
    name = "generic"
    language = "generic"
    extension = ".py"

    def generate(self, sanity: float, file_index: int) -> GeneratedFile:
        if native_generate_generic is not None:
            content, lc, cname = native_generate_generic(sanity, file_index)
            filename = f"generic/{cname.lower()}_{file_index}{self.extension}"
            return GeneratedFile(filename=filename, content=content, line_count=lc)

        parts: list[str] = []

        # pick which type of slop to generate
        slop_type = random.choice(["nested_if", "wrapper_hell", "massive_enum", "fake_tests", "mixed"])
        cname = naming.class_name(sanity)

        if slop_type == "nested_if":
            self._nested_if_else(parts, sanity, cname)
        elif slop_type == "wrapper_hell":
            self._wrapper_hell(parts, sanity, cname)
        elif slop_type == "massive_enum":
            self._massive_enum(parts, sanity, cname)
        elif slop_type == "fake_tests":
            self._fake_tests(parts, sanity, cname)
        else:
            self._nested_if_else(parts, sanity, cname)
            self._wrapper_hell(parts, sanity, cname)
            self._massive_enum(parts, sanity, cname)
            self._fake_tests(parts, sanity, cname)

        content = "".join(parts)
        filename = f"generic/{cname.lower()}_{file_index}{self.extension}"
        return GeneratedFile(filename=filename, content=content, line_count=line_count(content))

    def _nested_if_else(self, parts: list[str], sanity: float, cname: str) -> None:
        parts.append(f"# {comments.comment(sanity)}\n\n\n")
        func_name = naming.method_name(sanity)
        parts.append(
            f"def {func_name}({naming.var_name(sanity)}, {naming.var_name(sanity)}, {naming.var_name(sanity)}):\n"
        )
        parts.append(f'    """{comments.docstring(sanity, func_name)}"""\n')

        depth = random.randint(10, 20)
        for d in range(depth):
            indent = "    " * (d + 1)
            var = naming.var_name(sanity)
            condition = random.choice([
                f"{var} is not None",
                f"isinstance({var}, object)",
                f"len(str({var})) > 0",
                f"{var} != {var}",
                f"True",
                f"not False",
                f"bool({var}) or not bool({var})",
                f"hash({var}) == hash({var})",
                f"type({var}) == type({var})",
            ])
            parts.append(f"{indent}if {condition}:  # {comments.comment(sanity)}\n")
            parts.append(f"{indent}    {naming.var_name(sanity)} = None\n")

        for d in range(depth, 0, -1):
            indent = "    " * d
            parts.append(f"{indent}else:\n")
            parts.append(f"{indent}    pass  # {comments.comment(sanity)}\n")

        parts.append("    return None\n\n\n")

    def _wrapper_hell(self, parts: list[str], sanity: float, cname: str) -> None:
        parts.append(f"# {comments.comment(sanity)}\n\n")
        base_name = naming.method_name(sanity)
        layers = ["", "Internal", "InternalImpl", "InternalImplV2",
                  "InternalImplV2Final", "InternalImplV2FinalFinal",
                  "InternalImplV2FinalFinalForReal", "InternalImplV2FinalFinalForRealThisTime"]
        num_layers = random.randint(4, len(layers))

        for i in range(num_layers):
            suffix = layers[i]
            next_suffix = layers[i + 1] if i + 1 < num_layers else None
            fname = f"{base_name}{suffix}"
            params = ", ".join(naming.var_name(sanity) for _ in range(random.randint(1, 4)))

            parts.append(f"def {fname}({params}):\n")
            parts.append(f'    """{comments.docstring(sanity, fname)}"""\n')
            parts.append(f"    # {comments.comment(sanity)}\n")
            for _ in range(random.randint(1, 3)):
                parts.append(f"    {naming.var_name(sanity)} = None\n")

            if next_suffix is not None:
                next_name = f"{base_name}{next_suffix}"
                parts.append(f"    return {next_name}({params})\n\n\n")
            else:
                parts.append(f"    return None  # {comments.comment(sanity)}\n\n\n")

    def _massive_enum(self, parts: list[str], sanity: float, cname: str) -> None:
        parts.append(f"# {comments.comment(sanity)}\n")
        parts.append("from enum import Enum, auto\n\n\n")
        enum_name = cname + "Type"
        parts.append(f"class {enum_name}(Enum):\n")
        parts.append(f'    """{comments.docstring(sanity, enum_name)}"""\n\n')

        num_values = random.randint(50, 100)
        used_names = set()
        for i in range(num_values):
            if sanity > 0.5:
                parts_tokens = [
                    random.choice(naming.CORPORATE_PREFIXES).upper(),
                    random.choice(naming.CORPORATE_NOUNS).upper(),
                ]
                val_name = "_".join(parts_tokens) + f"_{i}"
            else:
                val_name = random.choice(naming.CHAOS_WORDS).upper() + f"_{i}"
            if val_name in used_names:
                val_name += f"_EXTRA_{i}"
            used_names.add(val_name)
            parts.append(f"    {val_name} = auto()  # {comments.comment(sanity)}\n")
        parts.append("\n\n")

    def _fake_tests(self, parts: list[str], sanity: float, cname: str) -> None:
        parts.append(f"# {comments.comment(sanity)}\n")
        parts.append("import unittest\n\n\n")

        test_class = f"Test{cname}"
        parts.append(f"class {test_class}(unittest.TestCase):\n")
        parts.append(f'    """{comments.docstring(sanity, test_class)}"""\n\n')

        num_tests = random.randint(10, 30)
        for i in range(num_tests):
            test_name = naming.method_name(sanity)
            parts.append(f"    def test_{test_name}_{i}(self):\n")
            parts.append(f"        # {comments.comment(sanity)}\n")

            num_asserts = random.randint(1, 5)
            for _ in range(num_asserts):
                assertion = random.choice([
                    "self.assertTrue(True)",
                    "self.assertFalse(False)",
                    "self.assertEqual(1, 1)",
                    "self.assertIsNone(None)",
                    "self.assertIsNotNone(object())",
                    "self.assertIn(1, [1, 2, 3])",
                    "self.assertEqual('a', 'a')",
                    "self.assertGreater(2, 1)",
                    "self.assertLess(1, 2)",
                    f"self.assertTrue(True)  # {comments.comment(sanity)}",
                ])
                parts.append(f"        {assertion}\n")
            parts.append("\n")

        parts.append("\nif __name__ == '__main__':\n")
        parts.append("    unittest.main()\n\n")
