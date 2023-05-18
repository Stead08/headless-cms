import { Application, Router } from "https://deno.land/x/oak/mod.ts";
import { proxy } from "https://deno.land/x/oak_http_proxy@2.1.0/mod.ts";

const app = new Application();
const router = new Router();

router.get("/api", proxy("http://localhost:8000"));
router.get("/:ctx", proxy((ctx) => new URL("http://localhost:3000/" + ctx.params.ctx)));

app.use(router.routes());
app.use(router.allowedMethods());

console.log("Proxy listening on port 3001");
await app.listen({ port: 3001 });

