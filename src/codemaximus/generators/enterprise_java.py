import random

from codemaximus.generators.base import SlopGenerator, GeneratedFile
from codemaximus import naming, comments
from codemaximus.native import line_count


class EnterpriseJavaGenerator(SlopGenerator):
    name = "enterprise_java"
    language = "java"
    extension = ".java"

    def generate(self, sanity: float, file_index: int) -> GeneratedFile:
        parts: list[str] = []
        cname = naming.class_name(sanity)
        pkg = naming.java_package(sanity)

        parts.append(f"package {pkg};\n\n")

        num_imports = random.randint(5, 15)
        for _ in range(num_imports):
            imp_pkg = naming.java_package(sanity)
            imp_class = naming.class_name(sanity)
            parts.append(f"import {imp_pkg}.{imp_class};\n")
        parts.append("\n")

        parts.append("/**\n")
        parts.append(f" * {comments.docstring(sanity, cname)}\n")
        parts.append(
            f" * @author {naming._blend(['Enterprise Code Generator', 'Architecture Team', 'Senior Staff Engineer'], ['a mass of vibes', 'the mass void', 'nobody'], sanity)}\n"
        )
        parts.append(" * @since 1.0.0\n")
        parts.append(" * @deprecated Since before it was written\n")
        parts.append(" */\n")

        extends = [naming.class_name(sanity) for _ in range(random.randint(0, 2))]
        implements = [naming.class_name(sanity) for _ in range(random.randint(1, 4))]

        decl = f"public class {cname}"
        if extends:
            decl += f" extends {extends[0]}"
        if implements:
            decl += f" implements {', '.join(implements)}"
        parts.append(f"{decl} {{\n\n")

        num_fields = random.randint(4, 12)
        fields: list[tuple[str, str]] = []
        java_types = [
            "String",
            "int",
            "long",
            "boolean",
            "double",
            "Object",
            "List<Object>",
            "Map<String, Object>",
            "Optional<String>",
            "AbstractFactory",
            "ServiceProvider",
            "CompletableFuture<Void>",
        ]
        for _ in range(num_fields):
            field_name = naming.var_name(sanity)
            field_type = random.choice(java_types)
            fields.append((field_type, field_name))
            parts.append(f"    private {field_type} {field_name};\n")
        parts.append("\n")

        parts.append(f"    public {cname}(")
        parts.append(", ".join(f"{t} {n}" for t, n in fields[:6]))
        parts.append(") {\n")
        for t, n in fields[:6]:
            parts.append(f"        this.{n} = {n};\n")
        parts.append("    }\n\n")

        for field_type, field_name in fields:
            cap = field_name[0].upper() + field_name[1:] if field_name else "X"
            parts.append("    /**\n")
            parts.append(f"     * Gets the {field_name}.\n")
            parts.append(f"     * @return the {field_name}\n")
            parts.append("     */\n")
            parts.append(f"    public {field_type} get{cap}() {{\n")
            parts.append(f"        return this.{field_name};\n")
            parts.append(f"    }}\n\n")
            parts.append("    /**\n")
            parts.append(f"     * Sets the {field_name}.\n")
            parts.append(f"     * @param {field_name} the {field_name} to set\n")
            parts.append("     */\n")
            parts.append(f"    public void set{cap}({field_type} {field_name}) {{\n")
            parts.append(f"        this.{field_name} = {field_name};\n")
            parts.append(f"    }}\n\n")

        num_methods = random.randint(3, 8)
        for _ in range(num_methods):
            mname = naming.method_name(sanity)
            return_type = random.choice(["void", "Object", "String", "boolean", "int"])
            params = ", ".join(
                f"{random.choice(java_types)} {naming.var_name(sanity)}"
                for _ in range(random.randint(0, 4))
            )
            for line in comments.block_comment(sanity, "    //"):
                parts.append(f"{line}\n")
            parts.append(f"    public {return_type} {mname}({params}) {{\n")

            num_body_lines = random.randint(2, 10)
            for _ in range(num_body_lines):
                vname = naming.var_name(sanity)
                parts.append(f"        Object {vname} = null; // {comments.comment(sanity)}\n")
            if return_type == "void":
                parts.append(f"        // {comments.comment(sanity)}\n")
            elif return_type == "boolean":
                parts.append(f"        return false; // {comments.comment(sanity)}\n")
            elif return_type == "int":
                parts.append(f"        return 0; // {comments.comment(sanity)}\n")
            else:
                parts.append(f"        return null; // {comments.comment(sanity)}\n")
            parts.append(f"    }}\n\n")

        num_inner = random.randint(1, 3)
        for _ in range(num_inner):
            inner_name = naming.class_name(sanity)
            parts.append(f"    public static class {inner_name} {{\n")
            for _ in range(random.randint(2, 5)):
                parts.append(f"        private Object {naming.var_name(sanity)};\n")
            parts.append(f"    }}\n\n")

        parts.append("}\n")

        content = "".join(parts)
        filename = f"java/{cname}_{file_index}{self.extension}"
        return GeneratedFile(
            filename=filename, content=content, line_count=line_count(content)
        )
