import { StartSimulatorPayload, useSimulator } from "@/store/simulator";
import { Button } from "../ui/button";
const StartSimButton = ({ cfg }: { cfg: StartSimulatorPayload }) => {
  const { start, loading } = useSimulator();
  return (
        <Button disabled={loading} onClick={() => start(cfg)}>
        {loading ? "Starting..." : "Start Simulator"}
        </Button>
    );
}

export default StartSimButton