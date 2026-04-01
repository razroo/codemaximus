import random

from codemaximus.generators.base import SlopGenerator, GeneratedFile
from codemaximus import naming, comments
from codemaximus.native import line_count


class GoSlopGenerator(SlopGenerator):
    name = "go_slop"
    language = "go"
    extension = ".go"

    def generate(self, sanity: float, file_index: int) -> GeneratedFile:
        parts: list[str] = []
        pkg = naming.go_package(sanity)
        struct_name = naming.class_name(sanity)

        parts.append(f"package {pkg}\n\n")

        go_imports = [
            '"fmt"',
            '"errors"',
            '"context"',
            '"sync"',
            '"time"',
            '"strings"',
            '"strconv"',
            '"io"',
            '"os"',
            '"log"',
            '"encoding/json"',
            '"net/http"',
            '"database/sql"',
            '"crypto/rand"',
            '"math/big"',
            '"bytes"',
        ]
        used = random.sample(go_imports, random.randint(4, 10))
        parts.append("import (\n")
        for imp in used:
            parts.append(f"\t{imp}\n")
        parts.append(")\n\n")

        parts.append("// suppress unused imports\n")
        parts.append("var (\n")
        for imp in used:
            name = imp.strip('"').split("/")[-1]
            if name == "io":
                parts.append(f"\t_ = {name}.ErrClosedPipe\n")
        parts.append("\t_ = fmt.Sprintf\n")
        parts.append("\t_ = errors.New\n")
        parts.append(")\n\n")

        parts.append(f"// {comments.comment(sanity)}\n")
        parts.append(f"type {struct_name} struct {{\n")
        go_types = [
            "interface{}",
            "string",
            "int",
            "int64",
            "bool",
            "float64",
            "[]byte",
            "map[string]interface{}",
            "chan struct{}",
            "context.Context",
            "*sync.Mutex",
            "error",
            "func() error",
            "[]interface{}",
            "*" + naming.class_name(sanity),
        ]
        num_fields = random.randint(10, 20)
        for _ in range(num_fields):
            fname = naming.var_name(sanity)
            fname = fname[0].upper() + fname[1:]
            ftype = random.choice(go_types)
            tag = f'`json:"{fname.lower()}" yaml:"{fname.lower()}" xml:"{fname.lower()}"`'
            parts.append(f"\t{fname} {ftype} {tag}\n")
        parts.append("}\n\n")

        parts.append(f"// New{struct_name} creates a new {struct_name}.\n")
        parts.append(f"// {comments.comment(sanity)}\n")
        parts.append(f"func New{struct_name}(ctx context.Context) (*{struct_name}, error) {{\n")
        parts.append("\tif ctx == nil {\n")
        parts.append(
            f'\t\treturn nil, errors.New("{naming.var_name(sanity)}: context cannot be nil")\n'
        )
        parts.append("\t}\n")
        parts.append(f"\treturn &{struct_name}{{}}, nil\n")
        parts.append("}\n\n")

        num_methods = random.randint(5, 12)
        for _ in range(num_methods):
            mname = naming.method_name(sanity)
            mname = mname[0].upper() + mname[1:]
            receiver = struct_name[0].lower()
            return_type = random.choice(
                [
                    "error",
                    "(interface{}, error)",
                    "(string, error)",
                    "(bool, error)",
                    "(int, error)",
                ]
            )

            parts.append(f"// {mname} {comments.comment(sanity)}\n")
            parts.append(
                f"func ({receiver} *{struct_name}) {mname}(ctx context.Context) {return_type} {{\n"
            )

            num_checks = random.randint(2, 6)
            for i in range(num_checks):
                err_var = f"err{i}" if sanity < 0.5 and i > 0 else "err"
                vname = naming.var_name(sanity)
                parts.append(f"\t{vname}, {err_var} := func() (interface{{}}, error) {{\n")
                parts.append(f"\t\t// {comments.comment(sanity)}\n")
                parts.append("\t\treturn nil, nil\n")
                parts.append("\t}()\n")
                parts.append(f"\tif {err_var} != nil {{\n")
                if "interface" in return_type or "string" in return_type:
                    parts.append(f"\t\treturn nil, {err_var}\n")
                elif "bool" in return_type:
                    parts.append(f"\t\treturn false, {err_var}\n")
                elif "int" in return_type:
                    parts.append(f"\t\treturn 0, {err_var}\n")
                else:
                    parts.append(f"\t\treturn {err_var}\n")
                parts.append("\t}\n")
                parts.append(f"\t_ = {vname} // {comments.comment(sanity)}\n\n")

            if return_type == "error":
                parts.append("\treturn nil\n")
            elif "bool" in return_type:
                parts.append("\treturn false, nil\n")
            elif "int" in return_type:
                parts.append("\treturn 0, nil\n")
            else:
                parts.append("\treturn nil, nil\n")
            parts.append("}\n\n")

        num_interfaces = random.randint(2, 4)
        for _ in range(num_interfaces):
            iname = naming.class_name(sanity)
            parts.append(f"// {iname} {comments.comment(sanity)}\n")
            parts.append(f"type {iname} interface {{\n")
            for _ in range(random.randint(3, 8)):
                imname = naming.method_name(sanity)
                imname = imname[0].upper() + imname[1:]
                parts.append(f"\t{imname}(ctx context.Context) error\n")
            parts.append("}\n\n")

        parts.append(f"// {comments.comment(sanity)}\n")
        parts.append(
            f"func ({struct_name[0].lower()} *{struct_name}) startWorkers(ctx context.Context) {{\n"
        )
        parts.append("\tch := make(chan interface{}, 100)\n")
        parts.append("\tvar wg sync.WaitGroup\n")
        num_workers = random.randint(3, 7)
        for _ in range(num_workers):
            parts.append("\twg.Add(1)\n")
            parts.append("\tgo func() {\n")
            parts.append("\t\tdefer wg.Done()\n")
            parts.append("\t\tfor {\n")
            parts.append("\t\t\tselect {\n")
            parts.append("\t\t\tcase <-ctx.Done():\n")
            parts.append("\t\t\t\treturn\n")
            parts.append(f"\t\t\tcase ch <- nil: // {comments.comment(sanity)}\n")
            parts.append("\t\t\t\ttime.Sleep(time.Millisecond)\n")
            parts.append("\t\t\t}\n")
            parts.append("\t\t}\n")
            parts.append("\t}()\n\n")
        parts.append("\t_ = ch\n")
        parts.append("\twg.Wait()\n")
        parts.append("}\n")

        content = "".join(parts)
        filename = f"go/{struct_name}_{file_index}{self.extension}"
        return GeneratedFile(
            filename=filename, content=content, line_count=line_count(content)
        )
