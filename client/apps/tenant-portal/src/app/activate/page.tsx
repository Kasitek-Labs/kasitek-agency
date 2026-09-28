import { Suspense } from "react";

import AcceptInviteScreen from "@/screens/accept-invite";

export default function Page() {
  return (
    <Suspense fallback={null}>
      <AcceptInviteScreen />
    </Suspense>
  );
}
