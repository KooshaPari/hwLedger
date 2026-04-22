// Fixture: canonical k6 user-story frontmatter for load tests.
// Not executed — harvested only.

import http from 'k6/http';
import { sleep } from 'k6';

/* @user-story
journey_id: fixture-k6-fleet-probe
title: Fleet probe endpoint sustains 100 rps
persona: SRE running fleet load tests
given: |
  A running hwledger-server with a seeded fleet of 10 hosts and mTLS disabled
  in the staging profile.
when:
  - ramp up to 100 virtual users over 30 seconds
  - each user GETs /api/fleet/probe every 1 second for 2 minutes
then:
  - p(95) response time stays under 200 ms
  - error rate stays under 0.5%
  - server heap delta after the run is < 100 MB
traces_to:
  - FR-TEL-001
  - FR-FLEET-002
record: false
blind_judge: skip
family: k6
backend: k6
*/
export const options = {
    vus: 100,
    duration: '2m',
};

export default function () {
    http.get('http://localhost:8080/api/fleet/probe');
    sleep(1);
}
