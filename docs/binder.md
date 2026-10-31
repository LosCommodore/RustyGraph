# Binder (mybinder.org): an explainer

*Researched October 2026. Funding and limits change, so check the linked sources for the current state.*

## What it is

[mybinder.org](https://mybinder.org) is a free public service. It takes a **public** Git repository, builds a container image from it, and starts a temporary JupyterLab session in the browser, with no account or login. Anyone with the link can run the notebooks in a repo without installing anything.

It runs on two open-source tools:

- **repo2docker** turns a repo into a Docker image by reading configuration files such as `requirements.txt`, `environment.yml`, `apt.txt`, `postBuild` or a `Dockerfile`.
- **BinderHub** runs the service: it builds and caches images and starts the sessions on Kubernetes.

You can also run your own BinderHub, for example inside a university.

## Basic features

- **One-click launch:** a link or badge like `https://mybinder.org/v2/gh/<user>/<repo>/HEAD?labpath=notebook.ipynb` opens the notebook directly.
- **Sources:** GitHub, GitLab, plain Git URLs, and others. The repo must be public.
- **Image caching:** the first launch of a commit builds the image, which can take minutes. Later launches of the same commit reuse it and start much faster. A new commit triggers a new build.
- **Temporary sessions:** nothing you change is saved. Download your work before you leave.

**Limits** (from the [usage guidelines](https://mybinder.readthedocs.io/en/latest/about/user-guidelines.html)):

| | |
|---|---|
| RAM | at least 1 GB, at most 2 GB. Going over restarts the kernel. |
| CPU | shared, "up to one cpu-hour" for heavy sessions |
| Idle shutdown | after more than 10 minutes of inactivity |
| Session length | aims for up to 6 hours, not guaranteed |
| Outgoing network | about 1 Mbit/s per session (to prevent abuse) |

## Rust on Binder

repo2docker has no built-in Rust support, so you install the toolchain and the kernel yourself. There are two ways:

1. **`postBuild` script:** keep repo2docker's default Python and Jupyter setup, and add a script that installs `rustup`, runs `cargo install evcxr_jupyter`, then `evcxr_jupyter --install`. This is the lighter option.
2. **`Dockerfile`:** full control, for example starting from the official `rust` image and installing JupyterLab and evcxr ([example gist](https://gist.github.com/vaaaaanquish/1f4fad6ae58645106d78dd6c2bed3702), [Polars example](https://github.com/pola-rs/polars/commit/ae9ff426f9845fcb334ad4f7c7376970e73b4372)). If a Dockerfile exists, repo2docker ignores all other config files. It also has to meet Binder's requirements (a non-root user, Jupyter installed), which is more work.

For this project you'd also need Graphviz (`graphviz` in `apt.txt`) for the pictures.

**What to expect with Rust:**

- **The first build is slow.** Compiling `evcxr_jupyter` from source takes several minutes. Later launches of the same commit are fast.
- **Running cells is slow.** evcxr compiles each cell, and Binder's CPU is shared. Expect a few seconds or more per cell; the first `:dep petgraph` cell takes noticeably longer than on a local machine.
- **2 GB of RAM** is enough for petgraph, but heavier crates can run out of memory while compiling.

## How they earn their money

They don't. mybinder.org is a **non-commercial service run by the Project Jupyter community**, with no ads, paid tier or user fees. In the project's words, it "relies on the generosity of volunteers and organizations" ([supporters page](https://mybinder.readthedocs.io/en/latest/about/supporters.html)).

- **A federation of clusters:** the service runs on several BinderHub clusters run by different organizations, each paying for its own hardware. Members currently include **2i2c** and **GESIS** (both listed as providing cloud credits). The **Berkeley Institute for Data Science (BIDS)** joined in April 2026 ([2i2c post](https://2i2c.org/blog/bids-mybinder-federation)).
- **How the members fund it:**
  - GESIS, a German social-science institute, pays through German Research Foundation (DFG) grants.
  - 2i2c says the fees from its member communities support this work ([2i2c Binder report, Oct 2025](https://2i2c.org/blog/binder-report/)). 2i2c sells managed JupyterHub hosting to research and education communities, and part of that revenue pays for its Binder cluster.
- **Past support:** Google cloud credits (2017–2022), OVHcloud (2019–2024), the Alan Turing Institute (2019–2023), Curvenote (2024), Simula Research Laboratory staff time (2015–2025), and a Moore Foundation grant (2017).
- **Currently,** the supporters page lists *no* organization giving more than $10,000 a year in direct funding. Capacity depends on what the federation members can afford.

The scale is large: 2i2c's cluster alone ran about 785,000 sessions in the first three quarters of 2025.

## Which data they use

- **No accounts:** Binder has no login, so it keeps no user profiles.
- **IP addresses** are logged and kept for **30 days**, only to detect abuse, according to the usage guidelines.
- **A public launch log:** every successful launch is published in the open [Events Archive](https://mybinder-sre.readthedocs.io/en/stable/analytics/events-archive.html) (daily files at archive.analytics.mybinder.org, since November 2018). Each entry records the **time (to the minute), where the repo is hosted, the repo and commit, and the status**. No user identity is included. In practice, **anyone can see how often a repo is launched**.
- **Your repo** is cloned and built into an image, and the build logs are visible during the launch. The repo is public anyway, so nothing new is exposed, but **never put secrets in it**. The docs say a repo used on Binder should not need any passwords or sensitive information.
- **Session contents** (files you upload, code you run) live only in the temporary container and are deleted when the session ends. On a shared public service, don't put anything confidential into a session.
- **Website analytics:** no full privacy policy for the website was found. The operations docs mention an analytics setup, but what it tracks wasn't confirmed. Check mybinder.org directly if this matters.

## Is it a fit for RustyGraph?

It would make the notebook genuinely runnable from a README badge, which a static GitHub Pages site can't do. The trade-offs are a slow first build, slow Rust cells on shared CPU, and no guarantee the service is available at any given moment. So the static Pages version stays the fast "read it" option, and Binder is an optional "run it yourself" link (the badge in the README).

### How it's set up here

Binder reads its configuration from the `.binder/` folder (when that folder exists, config files in the project root are ignored), so it stays out of the way of the Rust project:

| File | When it runs | What it does |
|---|---|---|
| `.binder/apt.txt` | image build | installs Graphviz for the pictures |
| `.binder/postBuild` | image build | installs Rust and the prebuilt `evcxr_jupyter` kernel, and downloads petgraph ahead of time |
| `.binder/start` | every session start | puts `cargo` on `PATH` so the kernel can compile cells |

To update the kernel, change `EVCXR_VERSION` in `postBuild`.

## Sources

- [mybinder.org usage guidelines](https://mybinder.readthedocs.io/en/latest/about/user-guidelines.html)
- [Binder introduction](https://mybinder.readthedocs.io/en/latest/introduction.html)
- [Supporting organizations](https://mybinder.readthedocs.io/en/latest/about/supporters.html)
- [The BinderHub Federation](https://binderhub.readthedocs.io/en/stable/federation/federation.html)
- [2i2c: Impact report from 2i2c's Binder federation instance](https://2i2c.org/blog/binder-report/)
- [2i2c: BIDS joins the mybinder.org federation](https://2i2c.org/blog/bids-mybinder-federation)
- [GESIS joins the Binder Federation](https://blog.jupyter.org/gesis-joins-the-binder-federation-8603d878ff77)
- [The Analytics Events Archive](https://mybinder-sre.readthedocs.io/en/stable/analytics/events-archive.html)
- [mybinder SRE analytics docs](https://mybinder-sre.readthedocs.io/en/latest/analytics/index.html)
- [Jupyter forum: logging the origin of launch events](https://discourse.jupyter.org/t/consciensciously-logging-the-origin-of-launch-events/147)
- [Using repo2docker](https://test-r2d.readthedocs.io/en/docstring-conda/usage.html)
- [Rust + Jupyter Dockerfile gist](https://gist.github.com/vaaaaanquish/1f4fad6ae58645106d78dd6c2bed3702)
- [Polars evcxr Dockerfile commit](https://github.com/pola-rs/polars/commit/ae9ff426f9845fcb334ad4f7c7376970e73b4372)
