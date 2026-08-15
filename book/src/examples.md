# Examples

Machine-first sequences. Prefix with `cargo run -p cxas-cli --` if `cxas` is not on `PATH`.

```text
cxas init --app-dir ./pilot
cxas lint --app-dir ./pilot --format json
cxas create --name demo --location us --project-id my-project
cxas apps list --location us --project-id my-project
cxas state --app-dir ./pilot --location us --project-id my-project
cxas actions init --app-dir ./pilot
cxas pull --app projects/p/locations/us/apps/a --location us --target-dir ./out --version-id v3
cxas deploy --app-dir ./pilot --location us --project-id my-project --channel-type web
cxas evals report --output-dir ./report
cxas migrate dfcx --source ./dfcx --location us --project-id my-project --yes
```

Apps persist in `.cxas/catalog.json` (override with `CXAS_CATALOG`). Full walkthrough: <https://yash-kavaiya.github.io/cxas-harness/examples.html>
