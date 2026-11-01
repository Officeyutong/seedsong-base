# M1 demo and listening acceptance

The automated music evidence is in [README.md](README.md). This document is
the checklist for the remaining manual acceptance, not a record of completed
browser or listening checks. All items below are pending until a reviewer
records an actual result. A successful build does not establish playback or
subjective music quality.

## Deployment record

| Field | Value |
| --- | --- |
| Public Cloudflare Pages URL | Pending |
| UI source commit | Pending |
| Published `deploy` commit | Pending |
| Pinned seedsong-base commit | Pending |
| Reviewer and date | Pending |
| Browser and operating system | Pending |

Push the UI source changes to `main`, wait for its build-and-publish workflow
to finish, and verify that Cloudflare Pages deploys the resulting `deploy`
commit. Record the revision actually served by the public site.

## Browser checks

- [ ] Open the public page and confirm that the cover and controls load without
  console errors.
- [ ] Paste the reference DNA below; generate music, play it, seek, and download
  both SVG and WAV. Confirm that the downloads open successfully.
- [ ] Generate random DNA and repeat the cover and playback checks.
- [ ] Confirm that sound settings default to music v2 and the rendered player
  displays `Music v2`.
- [ ] Download DNA after rendering. Confirm `music` is `seedsong-music/2` and
  `settings.musicVersion` is `2`.
- [ ] Select legacy v1. Confirm that the previous result becomes stale and WAV
  download is disabled. Render again and confirm `Music v1`,
  `seedsong-music/1`, and `settings.musicVersion: 1` in the corresponding places.
- [ ] Edit DNA or settings after rendering. Confirm that the DNA export describes
  the new inputs and version while the previous audio remains marked stale.
- [ ] Cancel an in-progress render; confirm that generating again works and
  loading controls recover.

Reference DNA:

```text
9a401c07e8553fa2710c0000000000000000209b62d000000000000000000000000
```

## Listening notes

For each DNA, compare v1 and v2 using the same timbre, sample rate and reverb.
Start with synthesized piano and zero reverb to hear the melody clearly, then
check DNA-selected timbres with the default reverb. Listen through the complete
piece and record the settings, version preference, audible defects and whether
another algorithm change is needed. Check melody flow, repetitions, phrase
endings, accompaniment balance, clicks and clipping. Include several random
seeds and retain their DNA. This is an internal listening check; M1 does not
require a formal community survey.

| Sample | DNA | Observations | Result |
| --- | --- | --- | --- |
| Reference | `9a401c07e8553fa2710c0000000000000000209b62d000000000000000000000000` | Pending | Pending |
| Large-leap regression | `5bf48d079c2680dd2a20fa36cb9e699caf567174410030c762975c0ff503ee31` | Pending | Pending |
| Repetition regression | `2aca91c192c493123d79aeb89ddab79e77f1cbf432df3c52339cbece0d26cabe` | Pending | Pending |
| Random samples | Record each generated DNA | Pending | Pending |

The [comparison commands](README.md#reproduce-and-listen) also generate local
v1/v2 WAV files. Do not mark listening complete from statistics alone.

## Submission links

- [ ] Public demo URL and deployed revision recorded above.
- [ ] Base and UI repository URLs and reviewed commits.
- [ ] Links to the [1,000-DNA report](quality-report.json),
  [native test output](workspace-tests.log), and [evidence manifest](evidence.json).
- [ ] Completed browser results and listening notes, including any open issues.
