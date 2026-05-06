"""CSV reader front-end for Lynxes Python APIs."""

from lynxes._lynxes import NodeFrame
from lynxes._lynxes import read_csv_native_py as _read_csv_native


def _require_pyarrow():
    try:
        import pyarrow as pa
        import pyarrow.compute as pc
        import pyarrow.csv as csv
    except ImportError as exc:
        raise ImportError(
            "lynxes.read_csv(engine='pyarrow') requires pyarrow. Install pyarrow to use it."
        ) from exc

    return pa, pc, csv


def _string_array_from_column(column, *, name):
    pa, pc, _ = _require_pyarrow()
    if column.null_count:
        raise ValueError(f"{name} cannot contain null values")
    return pc.cast(column.combine_chunks(), pa.string())


def _label_array(label, rows):
    pa, _, _ = _require_pyarrow()
    offsets = pa.array(range(rows + 1), type=pa.int32())
    values = pa.array([label] * rows, type=pa.string())
    return pa.ListArray.from_arrays(offsets, values)


def _singleton_label_array(column, *, name):
    pa, pc, _ = _require_pyarrow()
    if column.null_count:
        raise ValueError(f"{name} cannot contain null values")
    values = pc.cast(column.combine_chunks(), pa.string())
    offsets = pa.array(range(len(values) + 1), type=pa.int32())
    return pa.ListArray.from_arrays(offsets, values)


def _csv_table_to_node_frame(table, *, label=None, id_col=None, id_prefix=None):
    pa, _, _ = _require_pyarrow()
    column_names = table.column_names
    rows = table.num_rows

    if id_col is not None:
        if id_col not in column_names:
            raise ValueError(f"id_col {id_col!r} not found in CSV columns")
        id_array = _string_array_from_column(table[id_col], name=id_col)
    elif "_id" in column_names:
        id_array = _string_array_from_column(table["_id"], name="_id")
    else:
        prefix = id_prefix or "row"
        id_array = pa.array((f"{prefix}_{idx}" for idx in range(rows)), type=pa.string())

    if label is not None:
        label_array = _label_array(label, rows)
    elif "_label" in column_names:
        label_array = _singleton_label_array(table["_label"], name="_label")
    else:
        raise ValueError("read_csv requires label=... unless the CSV contains a _label column")

    arrays = [id_array, label_array]
    names = ["_id", "_label"]
    for name in column_names:
        if name in {"_id", "_label"}:
            continue
        if name.startswith("_"):
            raise ValueError(f"CSV column {name!r} is reserved by NodeFrame")
        arrays.append(table[name].combine_chunks())
        names.append(name)

    batch = pa.RecordBatch.from_arrays(arrays, names=names)
    return NodeFrame.from_arrow(batch)


def read_csv(
    path,
    *,
    label=None,
    id_col=None,
    id_prefix=None,
    engine="native",
    infer_schema_rows=None,
    batch_size=65536,
    has_header=True,
    delimiter=",",
    read_options=None,
    parse_options=None,
    convert_options=None,
):
    """Read a CSV file into a NodeFrame."""
    if engine == "native":
        if read_options is not None or parse_options is not None or convert_options is not None:
            raise ValueError("pyarrow CSV options require engine='pyarrow'")
        return _read_csv_native(
            path,
            label=label,
            id_col=id_col,
            id_prefix=id_prefix,
            infer_schema_rows=infer_schema_rows,
            batch_size=batch_size,
            has_header=has_header,
            delimiter=delimiter,
        )

    if engine != "pyarrow":
        raise ValueError("engine must be 'native' or 'pyarrow'")
    if infer_schema_rows is not None or batch_size != 65536 or not has_header or delimiter != ",":
        raise ValueError("native CSV options require engine='native'")

    _, _, csv = _require_pyarrow()
    table = csv.read_csv(
        path,
        read_options=read_options,
        parse_options=parse_options,
        convert_options=convert_options,
    )
    return _csv_table_to_node_frame(table, label=label, id_col=id_col, id_prefix=id_prefix)


def _node_frame_read_csv(
    _cls,
    path,
    *,
    label=None,
    id_col=None,
    id_prefix=None,
    engine="native",
    infer_schema_rows=None,
    batch_size=65536,
    has_header=True,
    delimiter=",",
    read_options=None,
    parse_options=None,
    convert_options=None,
):
    return read_csv(
        path,
        label=label,
        id_col=id_col,
        id_prefix=id_prefix,
        engine=engine,
        infer_schema_rows=infer_schema_rows,
        batch_size=batch_size,
        has_header=has_header,
        delimiter=delimiter,
        read_options=read_options,
        parse_options=parse_options,
        convert_options=convert_options,
    )


NodeFrame.read_csv = classmethod(_node_frame_read_csv)

__all__ = ["read_csv"]
