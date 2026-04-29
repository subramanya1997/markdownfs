import { Container, getContainer } from "@cloudflare/containers";

export interface Env {
  MdfsGateway: DurableObjectNamespace<MdfsGateway>;
  MARKDOWNFS_LISTEN: string;
  MARKDOWNFS_DATA_DIR: string;
  MARKDOWNFS_R2_BUCKET: string;
  MARKDOWNFS_R2_ENDPOINT: string;
  MARKDOWNFS_R2_ACCESS_KEY_ID: string;
  MARKDOWNFS_R2_SECRET_ACCESS_KEY: string;
  MARKDOWNFS_R2_REGION: string;
  MARKDOWNFS_R2_PREFIX: string;
}

export class MdfsGateway extends Container<Env> {
  defaultPort = 8080;
  sleepAfter = "10m";

  envVars = {
    MARKDOWNFS_LISTEN: this.env.MARKDOWNFS_LISTEN,
    MARKDOWNFS_DATA_DIR: this.env.MARKDOWNFS_DATA_DIR,
    MARKDOWNFS_R2_BUCKET: this.env.MARKDOWNFS_R2_BUCKET,
    MARKDOWNFS_R2_ENDPOINT: this.env.MARKDOWNFS_R2_ENDPOINT,
    MARKDOWNFS_R2_ACCESS_KEY_ID: this.env.MARKDOWNFS_R2_ACCESS_KEY_ID,
    MARKDOWNFS_R2_SECRET_ACCESS_KEY: this.env.MARKDOWNFS_R2_SECRET_ACCESS_KEY,
    MARKDOWNFS_R2_REGION: this.env.MARKDOWNFS_R2_REGION,
    MARKDOWNFS_R2_PREFIX: this.env.MARKDOWNFS_R2_PREFIX,
  };
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const container = getContainer(env.MdfsGateway, "mdfs-gateway");
    return container.fetch(request);
  },
};
