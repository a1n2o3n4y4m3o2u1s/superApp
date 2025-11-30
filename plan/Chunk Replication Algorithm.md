Chunk Replication Algorithm

Goals: Maintain file availability without full replicas, maximize geographic/ISP diversity, minimize redundant storage, survive churn.

Parameters
k = minimum fragments needed to reconstruct (e.g., 10)
m = total fragments produced (e.g., 30)
m_target = desired fragment-holders (≥ m)
repair_threshold = fraction triggering repair (e.g., 0.8 * m_target)
T_repair = periodic check interval

Upload & Distribution

Chunking: split file into 1-4 MiB chunks

Erasure Encode: produce m fragments with Reed-Solomon (k-of-m)

Merkle Indexing: compute fragment CIDs and chunk root, create manifest

Publish Manifest: signed DAG file:manifest event

Initial Seeder Selection: uploader + local mesh peers + trusted PoP peers

Repair & Rebalance
Periodically check each manifest availability
If available holders < repair_threshold:
missing = m_target - available
needed_fragments = pick_fragments_to_recreate(missing)
donors = select_donors(k)
if donors.count ≥ k:
reconstruct chunk from fragments
create new fragments
push to candidate peers
update manifest holder table

Candidate Selection Heuristics
Exclude peers in same /24 or ISP as existing holders
Prefer high uptime, free capacity, geographic diversity
Respect user quotas and opt-out flags

On-Demand Reconstruction
If k fragments unavailable, allow cooperative peer to reconstruct and stream

Proofs-of-Storage
Holders produce challenge-response proofs (Merkle inclusion + signed timestamp)

Garbage Collection
Unreferenced fragments removed after 30 days (unless pinned)
Manifest GC when no referencing DAG event exists