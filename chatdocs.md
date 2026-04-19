
On hf what  context size do the shwon vram reqs assume? Similarly for dense and moe what concurrency scaling factor (concurrent users vs steps) should I use

I’m checking how Hugging Face typically states VRAM requirements and how to translate that into concurrency for dense vs MoE serving, then I’ll turn it into a practical rule-of-thumb.
The useful split here is: HF “VRAM” figures are usually about fitting the weights, while real serving limits come from KV cache, which grows with total live tokens across all active sequences. I’m pulling one more source on MoE so I can give you a clean dense-vs-MoE scaling rule instead of hand-waving.
Thought for 25s
Usually, the VRAM numbers you see on Hugging Face are best read as a weights-fit number, not a full long-context serving number.
HF’s own optimization guide explicitly says that for short inputs, under about 1024 tokens, inference memory is dominated by loading the model weights, and gives the usual rough rule of ~2 GB per 1B params in bf16/fp16. That means the “shown VRAM” is basically assuming short prompts, tiny KV cache, and near-single-request conditions, not a packed long-context production workload. 

For real serving, the thing that starts killing you is KV cache, and that scales with the total live tokens across all active sequences. HF’s TGI docs note that larger max input token limits increase memory demand, and vLLM’s docs make the same point indirectly by exposing batch controls like max_num_seqs and max_num_batched_tokens, which trade off concurrency against memory. 

So the clean mental model is:

total VRAM ≈ weights + runtime overhead + KV cache

And for capacity planning, the KV term is what matters most:

KV cache load scales roughly with total active tokens = sum over all requests of (prompt tokens kept alive + generated tokens kept alive).

That means concurrency is not really “users.” It is closer to:

effective concurrency ≈ total live token budget / average live tokens per request

That is the right scaling factor to use.

For dense vs MoE:

Weights / fit-to-GPU: use total parameters. Even for MoE, you generally still need the full model state available in memory unless you are doing explicit offload tricks. HF notes that MoEs can have lower active compute but still incur higher memory cost because the whole model is present. 
Per-token decode compute / throughput: use active parameters, not total parameters. HF’s MoE docs and model pages are explicit that inference uses only a subset of parameters per token, so speed is tied much more to active params than total params. 
KV cache scaling: mostly treat it like a function of the attention stack and total live tokens, not “active experts.” In practice, MoE helps compute more than it helps KV pressure. So MoE often gives better tokens/sec than a dense model with similar total params, but does not magically fix long-context concurrency limits. 
Practical rule-of-thumb:
For HF card VRAM numbers, assume they mean something like:

1 request
shortish context unless the card explicitly says otherwise
no serious concurrency
no safety margin for production batching
For planning your own serving, use this instead:
Start with weights fit
Dense: size by actual loaded precision/quant.
MoE: size by total loaded model memory, not just active params. 
Reserve VRAM for KV
If using vLLM, this is literally what gpu_memory_utilization / kv_cache_memory_bytes are doing. 
Model concurrency as token-budget, not user-count
A decent first estimator is:
concurrent requests ≈ KV-token-budget / avg live tokens per request
where avg live tokens per request is roughly:
avg prompt length + avg generated length still resident
Dense vs MoE scaling factor
For throughput, MoE commonly behaves more like a dense model with parameter count near its active params than its total params.
For VRAM fit, MoE behaves closer to its total loaded params.
For context/concurrency, both dense and MoE are constrained heavily by KV cache, so use nearly the same concurrency math unless the model has special attention/KV tricks.
Bluntly: if you want one scaling factor to use in spreadsheets, use this:
Concurrent-user equivalent = total live-token budget / average live tokens per request

Then adjust compute expectations like this:

Dense model throughput factor ≈ based on full model size
MoE throughput factor ≈ based on active params
Dense and MoE context/concurrency memory factor ≈ based mostly on KV cache, so much closer than people assume
A good default approximation for planning is:
Dense: 1 user with 8k prompt + 1k output counts as about 9k live tokens
MoE: count it the same for memory/concurrency, but expect better decode throughput per token than a dense model with the same total params
If you are building a planner, the least-wrong shortcut is:
served concurrency ≈ min(compute-limited concurrency, KV-limited concurrency)

with:

compute-limited concurrency
Dense: scale from full loaded model size
MoE: scale from active params
KV-limited concurrency
Both: scale from total live tokens
That is the sane default.
If you want, I can turn this into a concrete calculator formula for your sheet, with inputs like VRAM, quant, context, average output, dense vs MoE, and an estimated concurrent-user count.

E.g. https://huggingface.co/majentik/Qwen3.6-35B-A3B-TurboQuant-MLX-3bit reports 18gb. What context is it for and similarly what scaling factor applies for a slider 

I’m pulling the actual model card and any attached config so I can tell you whether that 18 GB includes KV/cache headroom or is basically just the quantized weights. Then I’ll turn it into a concrete slider rule.
Thought for 11s
That 18 GB is not tied to a specific context length on the card. On that page, the author says “Approximate model size: ~18 GB”, lists “Weight Quantization 3-bit (~18 GB)”, and separately says the model requires approximately 18 GB of unified memory while recommending 24 GB+ Apple Silicon. That wording strongly indicates the 18 GB is basically the quantized model footprint / minimum fit number, not “18 GB at X tokens of context.” 
You can also see a tell on the same page: Hugging Face’s metadata block shows “MLX 15.2 GB” while the model card rounds it to ~18 GB. That is exactly the kind of spread you get when people are informally talking about weights plus some runtime cushion, not a rigorously specified context-length benchmark. The card never states a context size alongside the 18 GB figure.

The base model itself has a native context length of 262,144 tokens, and Qwen’s serving guidance says the default context is 262,144, while warning that if you hit OOM you should reduce the context window. In other words, the model architecture supports huge context, but that does not mean the 18 GB number includes running at 262K context in practice.

HF’s own LLM optimization guidance says that for short inputs, under about 1024 tokens, inference memory is dominated by loading the weights, and gives the rough rule of ~2 GB per 1B params in fp16/bf16. That is why, unless a model card explicitly says otherwise, you should read a VRAM or unified-memory number like this as “can load the model”, not “can serve long context at this memory.” 

So for your slider, the least-wrong assumption is:

18 GB = baseline load cost at effectively short context / negligible KV pressure

Then add context/concurrency as a second term.

For this specific model, the card says 35B total, 3B active per token, and the base model says 35B total with 3B activated. That means:

for memory-to-fit, treat it like a 35B-class model footprint
for decode compute / throughput, treat it more like a 3B-active MoE per token
for context and concurrency memory, treat it like a model whose KV/cache still grows with live tokens; MoE helps compute much more than it helps context memory. 
A practical slider model that will not lie to you:
Displayed memory = base_model_memory + context_overhead + concurrency_overhead

For this card:

base_model_memory: start at 18 GB
context_overhead: depends on active live tokens, not users
concurrency_overhead: depends on how many sequences are alive at once and how long they are
The correct scaling unit for the slider is not “users,” it is:
effective_live_tokens = concurrent_sequences × average_live_tokens_per_sequence

where average live tokens per sequence is roughly:

prompt tokens currently cached + generated tokens still resident

That is the quantity that should drive the context/concurrency slider.

For a simple UX slider, I would use this:

Context slider: scale by average live tokens per request
Concurrency slider: scale by simultaneous sequences
Internally combine them as
token_pressure = sequences × avg_live_tokens
Then estimate:
Dense model throughput slider factor ≈ use total dense model size
MoE throughput slider factor ≈ use active params, here about 3B active
Memory slider factor for both dense and MoE ≈ mostly use token_pressure, because KV/cache scales with live tokens either way 
Bluntly, for this model specifically:
If your slider is for “minimum memory to run”: anchor at 18 GB
If your slider is for “practical serving memory”: anchor at 24 GB minimum, because the card itself recommends 24 GB+
If your slider is for “throughput penalty from concurrency”: use MoE compute scaling closer to 3B active, not 35B total
If your slider is for “memory penalty from concurrency/context”: do not use 3B active; use live-token growth, because that is the dominant extra cost 
So the direct answer is:
The 18 GB on that page does not appear to assume any explicit context size. Treat it as a near-minimum model-load number, effectively short-context / low-concurrency. For a slider, use a base of 18 GB, then scale memory with total live tokens, and scale compute for MoE with 3B active params.

A good first-pass slider formula would be:

memory_estimate = 18 GB + k × (concurrent_sequences × avg_live_tokens / 1000)

where k is a calibration constant you fit empirically for your backend, because the card does not provide enough information to derive an exact KV-per-token number. The reason you need calibration is that this is an MLX + TurboQuant KV-cache build, and the card says TurboQuant compresses the KV cache but does not publish a concrete “GB per 1K tokens” number for this exact model card. 

If you want, I can give you a much more concrete slider spec next:
dense vs MoE formulas, plus default constants for 8K / 32K / 128K / 256K and 1–16 concurrent chats.

Gb of vram per 1k of context?

Same for the 2 seperate  concurrent user delins? Batch Size:

Log Scale
1
Inputs processed simultaneously per step (affects throughput & latency)

Sequence Length: 1,024
Max tokens per input; impacts KV cache (also affected by attention structure) & activations.

Concurrent Users:

Log Scale
1
Number of users running inference simultaneously (affects memory usage and per-user performance)


I’m pulling the serving-side pieces that matter here: how KV cache scales with sequence length, and how to separate “batch size this step” from “independent concurrent users.” Then I’ll turn it into slider math you can actually use.
Thought for 38s
For that Qwen3.6-35B-A3B family config, the seq-scaled KV/cache part is surprisingly light because it only uses full attention every 4th layer, i.e. 10 full-attention layers out of 40, with 2 KV heads and head_dim 256. 
So the clean estimate for per-sequence, per-token KV memory is:

KV bytes/token ≈ 2 × full_attention_layers × num_kv_heads × head_dim × bytes_per_element

For this model:

2 × 10 × 2 × 256 × bytes = 10,240 × bytes

That gives:

bf16/fp16 KV: 10,240 × 2 = 20,480 bytes/token
int8 KV: 10,240 × 1 = 10,240 bytes/token
int4 KV: 10,240 × 0.5 = 5,120 bytes/token
3-bit KV: 10,240 × 0.375 = 3,840 bytes/token
Converted to per 1K tokens of context, per live sequence:
bf16/fp16: about 19.5 MiB per 1K
int8 KV: about 9.8 MiB per 1K
int4 KV: about 4.9 MiB per 1K
3-bit KV: about 3.7 MiB per 1K
Those are the right first-pass numbers for your slider for this model’s sequence-scaled cache term. The reason this is lower than many dense models is the architecture mix of mostly linear attention with only periodic full attention, plus only 2 KV heads in full attention. 
One important caveat: this is the full-attention KV part. Because the model also has many linear_attention layers, there is additional state/runtime overhead, but it does not scale the same way as ordinary quadratic full-attention KV cache. So for a UI slider, the above values are the correct linear-with-context term, and then you leave some fixed runtime cushion on top.

So your memory model should be:

total VRAM ≈ base model memory + fixed runtime overhead + seq_scaled_cache

with:

seq_scaled_cache ≈ per_1k_context_GB × (sequence_length / 1024) × live_sequences

For example, using 3-bit KV:

seq_scaled_cache ≈ 0.00366 GB × (seq_len / 1024) × live_sequences

Using bf16 KV:

seq_scaled_cache ≈ 0.0195 GB × (seq_len / 1024) × live_sequences

That answers the first part.

Now the second part: your two sliders are not the same thing.

1) Sequence Length
This is straightforward:
Sequence Length should multiply the per-sequence KV/cache linearly.

For this model:

at 1,024 tokens:
bf16 KV: ~0.0195 GB per live sequence
3-bit KV: ~0.0037 GB per live sequence
at 8,192 tokens:
bf16 KV: ~0.156 GB per live sequence
3-bit KV: ~0.029 GB per live sequence
at 32,768 tokens:
bf16 KV: ~0.625 GB per live sequence
3-bit KV: ~0.117 GB per live sequence
at 262,144 tokens:
bf16 KV: ~5.0 GB per live sequence
3-bit KV: ~0.94 GB per live sequence
The base model supports 262,144 max positions, but that does not mean the published memory figure includes running at that context. Qwen’s config exposes the 262,144-token max context, while HF docs note that cache memory grows with generation length and long contexts require more memory. 
2) Concurrent Users
This should be modeled as live sequences, not “people on the website.”
For your slider, the right default is:

Concurrent Users ≈ simultaneous live sequences

Then:

persistent_memory_from_users ≈ per_sequence_cache × concurrent_users

So if each user has one active chat stream and similar context length:

memory scales basically linearly with Concurrent Users

For this Qwen model with 3-bit KV:

1 user @ 8K: ~0.029 GB
8 users @ 8K: ~0.23 GB
16 users @ 8K: ~0.47 GB
For bf16 KV:
1 user @ 8K: ~0.156 GB
8 users @ 8K: ~1.25 GB
16 users @ 8K: ~2.5 GB
That is the persistent cache side.
3) Batch Size
This is the one people usually model wrong.
Your Batch Size slider is not the same as Concurrent Users.

Concurrent Users = how many sequences are alive over time, which drives persistent KV/cache memory
Batch Size = how many sequences/tokens you process in the same engine step, which mostly affects throughput, latency, and transient prefill/activation memory
vLLM’s docs are explicit that max_num_seqs and max_num_batched_tokens trade off concurrency and KV-cache space. In practice, batch controls determine how many requests the engine packs together at once, while the cache burden comes from the live sequences/tokens being maintained. 
So for a UI model:

Use this split
Persistent memory
driven by Sequence Length × Concurrent Users
Transient step memory / performance
driven by Batch Size
especially important during prefill
much less directly tied to steady-state decode cache than Concurrent Users
For a simple planner, do this:
Memory slider math
live_sequences = concurrent_users
kv_mem_gb = kv_gb_per_1k × (sequence_length / 1024) × concurrent_users

Throughput/latency slider math
effective_decode_batch = min(batch_size, concurrent_users)
That gives you the right behavior:

if batch_size rises while concurrent_users=1, memory barely changes, but latency/throughput behavior changes little
if concurrent_users rises while batch_size=1, memory rises a lot, but throughput is poor because requests are not being packed
if both rise together, you get both more cache memory and more packed compute
What I would use in your UI
For this exact model, I’d expose:
Sequence Length
Use directly.
Concurrent Users
Interpret as:
number of simultaneously live sequences
Batch Size
Interpret as:
max sequences packed into the same decode/prefill step
Then compute:

KV memory = KV_GB_PER_1K × (SequenceLength / 1024) × ConcurrentUsers

For this model, set:

KV_GB_PER_1K_BF16 = 0.0195
KV_GB_PER_1K_INT8 = 0.0098
KV_GB_PER_1K_INT4 = 0.0049
KV_GB_PER_1K_3BIT = 0.0037
And for decode-side performance:
effective_batch = min(BatchSize, ConcurrentUsers)

That is the least-wrong slider behavior.

Blunt recommendation
For a planner UI, do not let Batch Size multiply persistent memory the same way Concurrent Users does. That is wrong.
Use:

persistent_vram = base_model + fixed_overhead + kv_per_1k × seq_len/1024 × concurrent_users

Then optionally add a smaller transient term like:

transient_prefill_overhead ∝ batch_size × seq_len

but keep it visually separate or much weaker, because it is backend-specific and not as stable a predictor as KV/cache. HF and vLLM both point to cache growth and batch-control tradeoffs, but they do not give a universal single constant for transient step memory across engines. 

If you want, next I can turn this into exact code for your sliders, including a dense-model fallback and a MoE-specific branch for Qwen-style hybrid attention.

I need #s for gemma4, qwen3.5 dense and moe variants too. Is it possible for a generic formula that delineated where abs necessary, e.g. Moe dense, rope vs x, mla vs x etc

I’m pulling the actual configs so I can give you numbers that map to architecture instead of guessing. The key split is KV-bearing attention style: standard/GQA, MLA, hybrid linear attention, and MoE only matters indirectly unless it changes the attention stack.
I’ve got the key part now: there is a generic formula, and MoE usually does not change the KV term unless the attention layout changes too. I’m checking whether “Qwen3.5” has an official public config family distinct from Qwen3/Qwen3.6 so I don’t accidentally mix generations.

Concur user vs batch size?

I’m narrowing it to the operational distinction now, because this is where planner UIs usually go wrong: one slider is about how many requests exist, the other is about how many the engine packs into one step.
Thought for 1m 37s
They are different.
Concurrent users means how many requests/sequences are alive at the same time. That mainly drives persistent memory usage, because each live sequence carries cache/state with it. In vLLM terms, more simultaneous sequences means more KV cache pressure; the docs explicitly note that lowering max_num_seqs or max_num_batched_tokens reduces concurrent requests in a batch and therefore reduces KV cache space required. 

Batch size means how many requests/tokens the engine chooses to process together in one scheduling step. That mainly drives throughput and latency behavior, plus some transient activation/prefill memory, but it is not the same thing as “number of users.” TGI exposes this separately with knobs like --max-concurrent-requests versus --max-batch-total-tokens, which is a strong clue that queueing/live requests and per-step packing are separate controls. 

The clean way to model it is:

Concurrent users → persistent memory term
Batch size → step packing / throughput term
They interact, but they are not interchangeable. 
A good planner approximation is:
persistent_VRAM ≈ base_model + KV_per_sequence × concurrent_users

and

effective_batch = min(batch_size, concurrent_users)

That second line matters because if you have 100 concurrent users but a batch size cap of 8, the engine is only packing up to 8 at once per step; the rest are waiting/queued/scheduled later. Conversely, if you set batch size to 64 but only have 3 active users, your effective batch is still just 3. This matches how vLLM and TGI separate batch token limits from concurrent request limits. 

So for your sliders:

Sequence Length should multiply the per-sequence KV/cache cost.
Concurrent Users should multiply the number of live sequences.
Batch Size should not multiply persistent VRAM the same way; it should mostly affect throughput, latency, and a smaller transient memory term. 
Bluntly: if your UI currently treats batch size and concurrent users as the same VRAM scaler, that is wrong.
A sane implementation is:

kv_mem = kv_gb_per_1k * (sequence_length / 1024) * concurrent_users
effective_batch = min(batch_size, concurrent_users)
Then use effective_batch for throughput/latency modeling, not for the main persistent cache term. 
For an actual chat product, “concurrent users” is also usually better labeled as simultaneously active generations or live sequences, because many “users” are idle and not consuming decode-time batch slots.

So compute vs space.complexities?

I’m mapping it into the cleanest split: what scales compute per step, what scales persistent memory, and what changes when attention is standard, MLA, or hybrid. Then I’ll reduce it to rules you can actually plug into a planner.
Thought for 8s
Yes. The right split is:
space complexity = mostly weights + persistent cache/state
compute complexity = mostly per-step math on the active batch

That is the planner-grade distinction. vLLM and TGI both separate controls for how many requests can exist versus how many tokens/sequences get packed into a batch step, which reflects exactly this split. 

For serving, think in two layers:

1. Persistent space complexity

Weights: mostly fixed once the model is loaded.
KV/cache or equivalent recurrent state: grows with live sequences and their retained context.
So the dominant memory term is usually:
Memory ≈ Weights + Runtime overhead + State_per_sequence × ConcurrentUsers
HF’s optimization guide notes that short-input inference is dominated by weights, while longer generation increases memory through the cache. 

2. Step compute complexity

Prefill compute grows with the amount of prompt work you do in the current scheduling window.
Decode compute grows with how many active sequences/tokens you process per step.
So the dominant compute term is more like:
Compute per step ≈ Cost_per_token × EffectiveBatch
where:

EffectiveBatch = min(BatchSize, ConcurrentUsers) for a simple planner.
That matches why vLLM exposes controls like max_num_seqs and max_num_batched_tokens, and why TGI separately exposes concurrent-request and batch-token limits. 

So in plain English:

Concurrent Users is primarily a space-complexity slider.
Batch Size is primarily a compute/throughput/latency slider.
They interact, but they are not the same thing. 
For architecture differences, the same principle holds:
Dense vs MoE: MoE often reduces compute per token because only a subset of experts is active, but it does not automatically reduce persistent context-state growth unless the attention/state architecture also changes.
Standard attention / GQA: state grows with cached K/V for the attention-bearing layers.
MLA / compressed-state variants: state per token can be much smaller.
Hybrid linear attention / mixed attention: state growth can be reduced versus full-attention-every-layer, because fewer layers carry ordinary KV cache. Your Qwen3.6 example exposes this directly in config: 40 layers total, but only 10 full-attention layers, 2 KV heads, head dim 256. 
So the generic planner formulas should be:
PersistentMemory
  ≈ WeightMemory
  + FixedRuntimeOverhead
  + StatePer1kTokens × (SequenceLength / 1024) × ConcurrentUsers
EffectiveBatch
  = min(BatchSize, ConcurrentUsers)
StepCompute
  ≈ TokenComputeCost × EffectiveBatch
And then for user-facing intuition:
Raise Sequence Length → memory rises roughly linearly with retained context state.
Raise Concurrent Users → memory rises roughly linearly.
Raise Batch Size → throughput usually improves up to a point, latency behavior changes, transient step memory rises some, but persistent cache memory should not rise the same way unless it also enables more simultaneously active sequences. 
The blunt version:
Space complexity answers: “How many chats can stay alive?”
Compute complexity answers: “How fast can I process the currently active work?”
That is the clean distinction you want in a sizing/planner tool.




