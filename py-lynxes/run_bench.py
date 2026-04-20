"""
Lynxes-only benchmark — graph build + core operations.
Usage:
    uv run python run_bench.py
"""

import os
import random
import statistics
import tempfile
import time
import numpy as np

import lynxes as gf


def lfr_complex_gf(n: int, mu: float = 0.1, avg_degree: int = 15, max_degree: int = 1000, seed: int = 42) -> gf.GraphFrame:
    """Numpy 벡터화를 활용한 고속 LFR 복합 그래프 생성기"""
    np.random.seed(seed)
    random.seed(seed)
    
    degrees = np.random.zipf(2.5, n)
    degrees = np.clip(degrees, 1, max_degree) 
    
    num_communities = max(10, int(n / 1000))
    comm_sizes = np.random.zipf(2.0, num_communities)
    comm_probs = comm_sizes / comm_sizes.sum()
    communities = np.random.choice(num_communities, size=n, p=comm_probs)

    external_degrees = np.random.binomial(degrees, mu)
    internal_degrees = degrees - external_degrees
    
    nodes = np.arange(n)

    def match_stubs(stub_list):
        np.random.shuffle(stub_list)
        if len(stub_list) % 2 != 0:
            stub_list = stub_list[:-1]
        if len(stub_list) == 0:
            return np.empty((0, 2), dtype=np.int32)
        return stub_list.reshape(-1, 2)

    edges_list = []
    
    for c in range(num_communities):
        c_nodes = nodes[communities == c]
        c_internal_degs = internal_degrees[communities == c]
        c_stubs = np.repeat(c_nodes, c_internal_degs)
        edges_list.append(match_stubs(c_stubs))
        
    external_stubs = np.repeat(nodes, external_degrees)
    edges_list.append(match_stubs(external_stubs))
    
    all_edges = np.vstack(edges_list)

    cities = ["Seoul", "Busan", "Incheon", "Daegu", "Daejeon"]
    person_statuses = ["active", "away", "inactive"]
    company_statuses = ["hiring", "stable", "scaling"]
    roles = ["Engineer", "Designer", "Manager", "Analyst"]
    projects = ["graph-runtime", "ai-core", "data-pipeline", "frontend-v2"]
    channels = ["coffee-chat", "study-group", "team-sync", "alumni"]
    
    is_company = np.random.rand(n) < 0.1
    batch_size = 500_000

    # 임시 파일에 그래프 데이터 작성
    with tempfile.NamedTemporaryFile(suffix=".gf", mode="w", delete=False, encoding="utf-8") as f:
        path = f.name
        node_buffer = []
        for i in range(n):
            if is_company[i]:
                city = random.choice(cities)
                status = random.choice(company_statuses)
                founded = random.randint(1990, 2024)
                node_buffer.append(f'(n{i}: Company {{ founded: {founded}, city: "{city}", status: "{status}" }})\n')
            else:
                city = random.choice(cities)
                status = random.choice(person_statuses)
                age = random.randint(20, 65)
                score = round(random.uniform(0.5, 0.99), 2)
                node_buffer.append(f'(n{i}: Person {{ age: {age}, score: {score}, city: "{city}", status: "{status}" }})\n')
            
            if len(node_buffer) >= batch_size:
                f.writelines(node_buffer)
                node_buffer.clear()
        if node_buffer:
            f.writelines(node_buffer)
            node_buffer.clear()
        
        f.write("\n")
            
        edge_buffer = []
        for src, dst in all_edges:
            src_is_comp = is_company[src]
            dst_is_comp = is_company[dst]
            
            if not src_is_comp and not dst_is_comp:
                if random.random() < 0.8:
                    channel = random.choice(channels)
                    weight = round(random.uniform(0.1, 1.0), 2)
                    since = random.randint(2015, 2025)
                    edge_buffer.append(f'n{src} -[KNOWS]-> n{dst} {{ since: {since}, weight: {weight}, channel: "{channel}" }}\n')
                else:
                    cohort = f"bootcamp-{random.randint(1, 10)}"
                    edge_buffer.append(f'n{src} -[MENTORED_THROUGH_BOOTCAMP]-> n{dst} {{ since: {random.randint(2018, 2024)}, cohort: "{cohort}" }}\n')
            
            elif not src_is_comp and dst_is_comp:
                if random.random() < 0.7:
                    role = random.choice(roles)
                    edge_buffer.append(f'n{src} -[WORKS_AT]-> n{dst} {{ role: "{role}", since: {random.randint(2010, 2025)}, status: "full-time" }}\n')
                else:
                    project = random.choice(projects)
                    edge_buffer.append(f'n{src} -[COLLABORATES_ON]-> n{dst} {{ project: "{project}", status: "pilot" }}\n')
                    
            elif src_is_comp and not dst_is_comp:
                role = random.choice(roles)
                edge_buffer.append(f'n{dst} -[WORKS_AT]-> n{src} {{ role: "{role}", since: {random.randint(2010, 2025)}, status: "contract" }}\n')
                
            else:
                edge_buffer.append(f'n{src} -[PARTNERS_WITH]-> n{dst} {{ since: {random.randint(2000, 2025)}, tier: "strategic" }}\n')

            if len(edge_buffer) >= batch_size:
                f.writelines(edge_buffer)
                edge_buffer.clear()
        
        if edge_buffer:
            f.writelines(edge_buffer)
            edge_buffer.clear()

    # 생성된 파일을 읽고 디스크 정리
    try:
        return gf.read_gf(path)
    finally:
        os.unlink(path)


def timeit(fn, reps=3):
    times = [0.0] * reps
    for i in range(reps):
        t0 = time.perf_counter()
        fn()
        times[i] = time.perf_counter() - t0
    return statistics.median(times)


def fmt(s):
    if s < 1e-3:
        return f"{s*1e6:.1f} us"
    if s < 1.0:
        return f"{s*1e3:.1f} ms"
    return f"{s:.2f} s"


REPS = 3

for n in [1000, 10000]:
    print(f"\n{'='*50}  n={n:,}")

    t0 = time.perf_counter()
    # 새로운 LFR 생성 로직 적용
    graph = lfr_complex_gf(n)
    t_build = time.perf_counter() - t0
    print(f"  graph build (LFR Numpy + read_gf)  : {fmt(t_build)}")

    t = timeit(lambda: (
        graph.lazy()
        .filter_nodes(gf.col("_id") == "n0")
        .expand(hops=2, direction="out")
        .collect()
    ), REPS)
    print(f"  2-hop expand                       : {fmt(t)}")

    t = timeit(lambda: graph.pagerank(), REPS)
    print(f"  pagerank                           : {fmt(t)}")

    t = timeit(lambda: graph.connected_components(), REPS)
    print(f"  connected_components               : {fmt(t)}")

    t = timeit(lambda: graph.shortest_path("n0", f"n{n//2}"), REPS)
    print(f"  shortest_path (n0 -> n{n//2})       : {fmt(t)}")