# 🧨 [Concept] Lynxes 콘셉트 문서 리팩토링 및 작성 가이드 (초강도/상세본)

**"우리가 왜 이 바퀴를 새로 발명했는지, 러스트 코어 레벨에서 증명하라"**

콘셉트 문서는 단순한 자기소개서가 아닙니다. 현재 `docs/index.md`나 개념 문서들이 보여주는 안일한 서술 방식을 전면 부정하고, 아키텍트와 시니어 엔지니어들을 설득할 수 있는 깊이로 문서를 리팩토링해야 합니다.

## 🚨 1. 현재 `docs/index.md`의 치명적 결함 및 해부

현재 `index.md`의 3~4번 라인을 보면 다음과 같이 적혀 있습니다.
> "Lynxes is a graph analytics engine built natively on Apache Arrow. It stores node and edge data in Arrow RecordBatch objects..."

또한 64번 라인의 `## What Lynxes Gives You` 섹터는 그저 일반적인 장점만 텍스트로 나열한 팜플렛에 불과합니다. 노련한 시스템 개발자는 이런 모호한 문구를 보고 도구를 도입하지 않습니다.

**결함 목록:**
1. **가장 중요한 차별점의 부재:** "왜 Polars가 아니라 Arrow-rs를 직접 썼는가?", "왜 기존의 잘 나가는 NetworkX/igraph를 안 쓰고 굳이 Rust로 엔진을 짰는가?"에 대한 기술적 'Why'가 완벽히 누락되었습니다.
2. **블랙박스화된 내부 엔진:** `RecordBatch` 객체를 쓴다고만 했지, 이것이 메모리 상에서 어떻게 인접성(Cache Locality)을 구조적으로 획득하는지 증명하지 않습니다.

## 🛠 2. 즉각적인 리팩토링 지시 및 작성 규칙

### Rule 1: 'Polars 배제 상황'과 'Memory Layout' 철학 문서 신설
`concepts/arrow-csr.md` 등 신규 아키텍처 문서 최상단에, 다른 기술들을 래핑(Wrapping)만 하지 않은 이유를 단호하고 공격적으로 명시하십시오.
*   **추가할 핵심 논지:** "Pandas가 느려서 Polars가 Arrow 기반으로 새로 짜여진 것처럼, Lynxes도 Graph 분석의 한계를 부수기 위해 Polars 시스템의 오버헤드조차 배제하고 Arrow-rs 코어스택의 RecordBatch 단에서 직접 CSR(Compressed Sparse Row) 인덱스를 통제해야만 했다."
*   **다이어그램 강제:** `NodeFrame`과 `EdgeFrame`이 각각 어떤 Arrow 스키마(Offsets, Indices, Values 배열 레이아웃)를 갖는지 메모리 블록 다이어그램(Mermaid)으로 명확히 시각화하세요.

### Rule 2: Lazy Execution(지연 평가) 파이프라인의 시각적 해체
`concepts/lazy-execution.md` 문서는 코드가 아니라 '엔진 구조의 심장'을 보여줘야 합니다.
*   사용자가 `.collect()`를 호출했을 때, 시스템 내부의 지연 평가 파이프라인에서 어떤 최적화(Optimizer Passes)가 일어나는지 구체적으로 명시하세요.
*   **Mermaid.js 도입 지시:** 가비지 컬렉팅 및 연산 평가 파이프라인(`LogicalPlan` -> `PhysicalPlan` -> Rayon `Executor`)을 Flowchart 형태로 그려서 무의미한 CPU 낭비가 어떻게 억제되는지 시각적으로 증명하세요.

### Rule 3: Trade-offs(트레이드오프)의 투명성 강제 서술
Lynxes가 그래프를 읽고 탐색할 때 얻는 어마어마한 성능적 이점을 적었다면, 시스템 구조상 포기할 수밖에 없었던 부분도 정직하게 선언하십시오.
*   **명시할 내용 가이드:** "Lynxes는 CSR 타겟팅으로 인해 O(degree)의 타의 추종을 불허하는 압도적인 읽기/탐색 속도를 보장합니다. 하지만 그 대가로, 노드와 엣지가 끊임없이 실시간 갱신되는 스트리밍 목적(Streaming Mutate)에는 완전히 부적합합니다. 이는 불변성(Immutable) 프레임을 전면 재생성해야 하는 Arrow Columnar 구조의 피할 수 없는 한계점입니다."
*   독자는 이 한계를 투명하게 읽었을 때 비로소, 앞선 장점 어필을 100% 신뢰하게 됩니다.

## 📝 3. (예시) 이렇게 전면 교체하십시오.

**[AS-IS] (과거의 밋밋한 설명)**
> Lynxes uses Lazy queries to make things faster. Nothing happens until you call `collect()`.

**[TO-BE] (수정되어야 할 강도 높은 원리 설명)**
> ### Lazy Execution과 Traversal Pruning의 내부 메커니즘
> Lynxes의 `lazy()` API는 파이썬에서 호출되는 즉시 그 어떤 I/O 연산도 수행하지 않습니다. 대신 유저 쿼리는 순수한 AST(초상형 구문 트리) 형태의 `LogicalPlan` 객체로 컨디션 스택에 쌓입니다.
> 
> 마침내 `.collect()`가 호출되는 순간, 러스트 코어 엔진 내부의 **Optimizer**가 공격적으로 개입합니다. 만약 쿼리 논리상 `Predicate Pushdown(조건절 하위 푸시다운)`이 가능하거나 불필요한 노드 방문(Traversal)이 감지될 경우, 물리적 엔진(Physical Plan)으로 진입하기 직전에 Rayon 스레드 풀의 할당을 원천적으로 차단하여 메모리와 CPU 소모를 극도로 억제합니다. 즉, 쓰이지 않을 데이터는 애초에 메모리 버퍼 위에 오르지 않습니다.
