import dataclasses
import json
import sys
from pathlib import Path

from perf import make_datasets, prepare_dataset, sha256_path, timed_sample


def main() -> None:
    proof = Path(sys.argv[1])
    binaries = {
        "before": proof / "fz-before",
        "after": proof / "fz-after",
        "control": proof / "fz-control",
        "fzy": proof / "fzy" / "fzy",
    }
    dataset = make_datasets()["small"]
    prepared = prepare_dataset(proof / "fixtures", dataset)
    query = next(query for query in dataset.queries if query.name == "longer")
    hashes = {label: sha256_path(path) for label, path in binaries.items()}
    if hashes["before"] != hashes["control"] or hashes["before"] == hashes["after"]:
        raise ValueError("expected an identical baseline control and different candidate")
    for path in binaries.values():
        timed_sample(prepared, path, query, "default")
    labels = list(binaries)
    rows = []
    for index in range(40):
        offset = index % len(labels)
        order = labels[offset:] + labels[:offset]
        rows.append({
            label: dataclasses.asdict(timed_sample(prepared, binaries[label], query, "default"))
            for label in order
        })
    print(json.dumps({"fixture": prepared.sha256, "binaries": hashes, "rows": rows}))


if __name__ == "__main__":
    main()
